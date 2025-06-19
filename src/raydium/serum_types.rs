use crate::raydium::serum_error::{DexErrorCode, DexResult};
use anyhow::{Error, anyhow};
use arrayref::mut_array_refs;
use bytemuck::{
    Pod, Zeroable, cast, cast_slice, cast_slice_mut, from_bytes_mut, try_cast_slice_mut,
    try_from_bytes_mut,
};
use enumflags2::BitFlags;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use safe_transmute::TriviallyTransmutable;
use solana_sdk::account_info::AccountInfo;
use solana_sdk::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::rent::Rent;
use std::cell::RefMut;
use std::convert::identity;
use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};

pub const ACCOUNT_TAIL_PADDING: &[u8; 7] = b"padding";
pub const ACCOUNT_HEAD_PADDING: &[u8; 5] = b"serum";
#[derive(Debug, Copy, Clone)]
pub enum Side {
    Bid = 0,
    Ask = 1,
}

#[derive(Copy, Clone, BitFlags, Debug, Eq, PartialEq)]
#[repr(u64)]
pub enum AccountFlag {
    Initialized = 1u64 << 0,
    Market = 1u64 << 1,
    OpenOrders = 1u64 << 2,
    RequestQueue = 1u64 << 3,
    EventQueue = 1u64 << 4,
    Bids = 1u64 << 5,
    Asks = 1u64 << 6,
    Disabled = 1u64 << 7,
    Closed = 1u64 << 8,
    Permissioned = 1u64 << 9,
    CrankAuthorityRequired = 1u64 << 10,
}

#[derive(Copy, Clone)]
#[cfg_attr(target_endian = "little", derive(Debug))]
#[repr(packed)]
pub struct MarketState {
    // 0
    pub account_flags: u64, // Initialized, Market

    // 1
    pub own_address: [u64; 4],

    // 5
    pub vault_signer_nonce: u64,
    // 6
    pub coin_mint: [u64; 4],
    // 10
    pub pc_mint: [u64; 4],

    // 14
    pub coin_vault: [u64; 4],
    // 18
    pub coin_deposits_total: u64,
    // 19
    pub coin_fees_accrued: u64,

    // 20
    pub pc_vault: [u64; 4],
    // 24
    pub pc_deposits_total: u64,
    // 25
    pub pc_fees_accrued: u64,

    // 26
    pub pc_dust_threshold: u64,

    // 27
    pub req_q: [u64; 4],
    // 31
    pub event_q: [u64; 4],

    // 35
    pub bids: [u64; 4],
    // 39
    pub asks: [u64; 4],

    // 43
    pub coin_lot_size: u64,
    // 44
    pub pc_lot_size: u64,

    // 45
    pub fee_rate_bps: u64,
    // 46
    pub referrer_rebates_accrued: u64,
}
pub trait QueueHeader: Pod {
    type Item: Pod + Copy;

    fn head(&self) -> u64;
    fn set_head(&mut self, value: u64);
    fn count(&self) -> u64;
    fn set_count(&mut self, value: u64);

    fn incr_event_id(&mut self);
    fn decr_event_id(&mut self, n: u64);
}

pub struct Queue<'a, H: QueueHeader> {
    header: RefMut<'a, H>,
    buf: RefMut<'a, [H::Item]>,
}

impl<'a, H: QueueHeader> Queue<'a, H> {
    pub fn new(header: RefMut<'a, H>, buf: RefMut<'a, [H::Item]>) -> Self {
        Self { header, buf }
    }

    #[inline]
    pub fn len(&self) -> u64 {
        self.header.count()
    }

    #[inline]
    pub fn full(&self) -> bool {
        self.header.count() as usize == self.buf.len()
    }

    #[inline]
    pub fn empty(&self) -> bool {
        self.header.count() == 0
    }

    #[inline]
    pub fn push_back(&mut self, value: H::Item) -> Result<(), H::Item> {
        if self.full() {
            return Err(value);
        }
        let slot = ((self.header.head() + self.header.count()) as usize) % self.buf.len();
        self.buf[slot] = value;

        let count = self.header.count();
        self.header.set_count(count + 1);

        self.header.incr_event_id();
        Ok(())
    }

    #[inline]
    pub fn peek_front(&self) -> Option<&H::Item> {
        if self.empty() {
            return None;
        }
        Some(&self.buf[self.header.head() as usize])
    }

    #[inline]
    pub fn peek_front_mut(&mut self) -> Option<&mut H::Item> {
        if self.empty() {
            return None;
        }
        Some(&mut self.buf[self.header.head() as usize])
    }

    #[inline]
    pub fn pop_front(&mut self) -> Result<H::Item, ()> {
        if self.empty() {
            return Err(());
        }
        let value = self.buf[self.header.head() as usize];

        let count = self.header.count();
        self.header.set_count(count - 1);

        let head = self.header.head();
        self.header.set_head((head + 1) % self.buf.len() as u64);

        Ok(value)
    }

    #[inline]
    pub fn revert_pushes(&mut self, desired_len: u64) -> DexResult<()> {
        let len_diff = self.header.count() - desired_len;
        self.header.set_count(desired_len);
        self.header.decr_event_id(len_diff);
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = &H::Item> {
        QueueIterator {
            queue: self,
            index: 0,
        }
    }
}

struct QueueIterator<'a, 'b, H: QueueHeader> {
    queue: &'b Queue<'a, H>,
    index: u64,
}

impl<'a, 'b, H: QueueHeader> Iterator for QueueIterator<'a, 'b, H> {
    type Item = &'b H::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.queue.len() {
            None
        } else {
            let item = &self.queue.buf
                [(self.queue.header.head() + self.index) as usize % self.queue.buf.len()];
            self.index += 1;
            Some(item)
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct EventQueueHeader {
    account_flags: u64, // Initialized, EventQueue
    head: u64,
    count: u64,
    seq_num: u64,
}
unsafe impl Zeroable for EventQueueHeader {}
unsafe impl Pod for EventQueueHeader {}

unsafe impl TriviallyTransmutable for EventQueueHeader {}
#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct Event {
    event_flags: u8,
    owner_slot: u8,

    fee_tier: u8,

    _padding: [u8; 5],

    native_qty_released: u64,
    native_qty_paid: u64,
    native_fee_or_rebate: u64,

    order_id: u128,
    pub owner: [u64; 4],
    client_order_id: u64,
}
unsafe impl Zeroable for Event {}
unsafe impl Pod for Event {}

unsafe impl TriviallyTransmutable for Event {}
#[derive(Copy, Clone, BitFlags, Debug)]
#[repr(u8)]
enum EventFlag {
    Fill = 0x1,
    Out = 0x2,
    Bid = 0x4,
    Maker = 0x8,
    ReleaseFunds = 0x10,
}

impl EventFlag {
    #[inline]
    fn from_side(side: Side) -> BitFlags<Self> {
        match side {
            Side::Bid => EventFlag::Bid.into(),
            Side::Ask => BitFlags::empty(),
        }
    }

    #[inline]
    fn flags_to_side(flags: BitFlags<Self>) -> Side {
        if flags.contains(EventFlag::Bid) {
            Side::Bid
        } else {
            Side::Ask
        }
    }
}

impl Event {
    #[inline(always)]
    pub fn new(view: EventView) -> Self {
        match view {
            EventView::Fill {
                side,
                maker,
                native_qty_paid,
                native_qty_received,
                native_fee_or_rebate,
                order_id,
                owner,
                owner_slot,
                fee_tier,
                client_order_id,
            } => {
                let maker_flag = if maker {
                    BitFlags::from_flag(EventFlag::Maker).bits()
                } else {
                    0
                };
                let event_flags =
                    (EventFlag::from_side(side) | EventFlag::Fill).bits() | maker_flag;
                Event {
                    event_flags,
                    owner_slot,
                    fee_tier: fee_tier.into(),

                    _padding: Zeroable::zeroed(),

                    native_qty_released: native_qty_received,
                    native_qty_paid,
                    native_fee_or_rebate,

                    order_id,
                    owner,

                    client_order_id: client_order_id.map_or(0, NonZeroU64::get),
                }
            }

            EventView::Out {
                side,
                release_funds,
                native_qty_unlocked,
                native_qty_still_locked,
                order_id,
                owner,
                owner_slot,
                client_order_id,
            } => {
                let release_funds_flag = if release_funds {
                    BitFlags::from_flag(EventFlag::ReleaseFunds).bits()
                } else {
                    0
                };
                let event_flags =
                    (EventFlag::from_side(side) | EventFlag::Out).bits() | release_funds_flag;
                Event {
                    event_flags,
                    owner_slot,
                    fee_tier: 0,

                    _padding: Zeroable::zeroed(),

                    native_qty_released: native_qty_unlocked,
                    native_qty_paid: native_qty_still_locked,
                    native_fee_or_rebate: 0,

                    order_id,
                    owner,
                    client_order_id: client_order_id.map_or(0, NonZeroU64::get),
                }
            }
        }
    }

    #[inline(always)]
    pub fn as_view(&self) -> DexResult<EventView> {
        let flags = BitFlags::from_bits(self.event_flags).unwrap();
        let side = EventFlag::flags_to_side(flags);
        let client_order_id = NonZeroU64::new(self.client_order_id);
        if flags.contains(EventFlag::Fill) {
            let allowed_flags = {
                use EventFlag::*;
                Fill | Bid | Maker
            };

            return Ok(EventView::Fill {
                side,
                maker: flags.contains(EventFlag::Maker),
                native_qty_paid: self.native_qty_paid,
                native_qty_received: self.native_qty_released,
                native_fee_or_rebate: self.native_fee_or_rebate,

                order_id: self.order_id,
                owner: self.owner,

                owner_slot: self.owner_slot,
                fee_tier: self.fee_tier.try_into().unwrap(),
                client_order_id,
            });
        }
        Ok(EventView::Out {
            side,
            release_funds: flags.contains(EventFlag::ReleaseFunds),
            native_qty_unlocked: self.native_qty_released,
            native_qty_still_locked: self.native_qty_paid,

            order_id: self.order_id,
            owner: self.owner,

            owner_slot: self.owner_slot,
            client_order_id,
        })
    }
}

impl QueueHeader for EventQueueHeader {
    type Item = Event;

    fn head(&self) -> u64 {
        self.head
    }
    fn set_head(&mut self, value: u64) {
        self.head = value;
    }
    fn count(&self) -> u64 {
        self.count
    }
    fn set_count(&mut self, value: u64) {
        self.count = value;
    }
    fn incr_event_id(&mut self) {
        self.seq_num += 1;
    }
    fn decr_event_id(&mut self, n: u64) {
        self.seq_num -= n;
    }
}

pub type EventQueue<'a> = Queue<'a, EventQueueHeader>;

#[cfg(target_endian = "little")]
unsafe impl Pod for MarketState {}
impl MarketState {
    pub fn load_event_queue_mut<'a>(&self, queue: &'a AccountInfo) -> DexResult<EventQueue<'a>> {
        let (header, buf) = strip_header::<EventQueueHeader, Event>(queue, false)?;
        Ok(Queue { header, buf })
    }

    pub fn load<'a>(
        market_account: &'a AccountInfo,
        program_id: &Pubkey,
        allow_disabled: bool,
    ) -> DexResult<RefMut<'a, Self>> {
        let mut account_data: RefMut<'a, [u8]>;
        let state: RefMut<'a, Self>;

        account_data = RefMut::map(market_account.try_borrow_mut_data().unwrap(), |data| *data);
        check_account_padding(&mut account_data)?;
        state = RefMut::map(account_data, |data| {
            from_bytes_mut(cast_slice_mut(
                check_account_padding(data).unwrap_or_else(|_| unreachable!()),
            ))
        });

        Ok(state)
    }
}

#[derive(Copy, Clone)]
pub struct OpenOrders {
    pub account_flags: u64, // Initialized, OpenOrders
    pub market: [u64; 4],
    pub owner: [u64; 4],

    pub native_coin_free: u64,
    pub native_coin_total: u64,

    pub native_pc_free: u64,
    pub native_pc_total: u64,

    pub free_slot_bits: u128,
    pub is_bid_bits: u128,
    pub orders: [u128; 128],
    // Using Option<NonZeroU64> in a pod type requires nightly
    pub client_order_ids: [u64; 128],
    pub referrer_rebates_accrued: u64,
}
unsafe impl Pod for OpenOrders {}
unsafe impl Zeroable for OpenOrders {}

impl OpenOrders {
    fn check_flags(&self) -> DexResult {
        let flags = BitFlags::from_bits(self.account_flags)
            .map_err(|_| DexErrorCode::InvalidMarketFlags)
            .unwrap();
        let required_flags = AccountFlag::Initialized | AccountFlag::OpenOrders;
        if flags != required_flags {
            Err(DexErrorCode::WrongOrdersAccount).unwrap()
        }
        Ok(())
    }

    fn init(&mut self, market: &[u64; 4], owner: &[u64; 4]) -> DexResult<()> {
        self.account_flags = (AccountFlag::Initialized | AccountFlag::OpenOrders).bits();
        self.market = *market;
        self.owner = *owner;
        self.native_coin_total = 0;
        self.native_coin_free = 0;
        self.native_pc_total = 0;
        self.native_pc_free = 0;
        self.free_slot_bits = std::u128::MAX;
        Ok(())
    }

    fn credit_locked_coin(&mut self, native_coin_amount: u64) {
        self.native_coin_total = self
            .native_coin_total
            .checked_add(native_coin_amount)
            .unwrap();
    }

    fn credit_locked_pc(&mut self, native_pc_amount: u64) {
        self.native_pc_total = self.native_pc_total.checked_add(native_pc_amount).unwrap();
    }

    fn lock_free_coin(&mut self, native_coin_amount: u64) {
        self.native_coin_free = self
            .native_coin_free
            .checked_sub(native_coin_amount)
            .unwrap();
    }

    fn lock_free_pc(&mut self, native_pc_amount: u64) {
        self.native_pc_free = self.native_pc_free.checked_sub(native_pc_amount).unwrap();
    }

    pub fn unlock_coin(&mut self, native_coin_amount: u64) {
        self.native_coin_free = self
            .native_coin_free
            .checked_add(native_coin_amount)
            .unwrap();
        assert!(self.native_coin_free <= self.native_coin_total);
    }

    pub fn unlock_pc(&mut self, native_pc_amount: u64) {
        self.native_pc_free = self.native_pc_free.checked_add(native_pc_amount).unwrap();
        assert!(self.native_pc_free <= self.native_pc_total);
    }

    fn slot_is_free(&self, slot: u8) -> bool {
        let slot_mask = 1u128 << slot;
        self.free_slot_bits & slot_mask != 0
    }

    #[inline]
    fn iter_filled_slots(&self) -> impl Iterator<Item = u8> {
        struct Iter {
            bits: u128,
        }
        impl Iterator for Iter {
            type Item = u8;
            #[inline(always)]
            fn next(&mut self) -> Option<Self::Item> {
                if self.bits == 0 {
                    None
                } else {
                    let next = self.bits.trailing_zeros();
                    let mask = 1u128 << next;
                    self.bits &= !mask;
                    Some(next as u8)
                }
            }
        }
        Iter {
            bits: !self.free_slot_bits,
        }
    }

    #[inline]
    fn orders_with_client_ids(&self) -> impl Iterator<Item = (NonZeroU64, u128, Side)> + '_ {
        self.iter_filled_slots().filter_map(move |slot| {
            let client_order_id = NonZeroU64::new(self.client_order_ids[slot as usize])?;
            let order_id = self.orders[slot as usize];
            let side = self.slot_side(slot).unwrap();
            Some((client_order_id, order_id, side))
        })
    }

    pub fn slot_side(&self, slot: u8) -> Option<Side> {
        let slot_mask = 1u128 << slot;
        if self.free_slot_bits & slot_mask != 0 {
            None
        } else if self.is_bid_bits & slot_mask != 0 {
            Some(Side::Bid)
        } else {
            Some(Side::Ask)
        }
    }

    pub fn remove_order(&mut self, slot: u8) -> DexResult {
        let slot_mask = 1u128 << slot;
        self.orders[slot as usize] = 0;
        self.client_order_ids[slot as usize] = 0;
        self.free_slot_bits |= slot_mask;
        self.is_bid_bits &= !slot_mask;

        Ok(())
    }

    fn add_order(&mut self, id: u128, side: Side) -> DexResult<u8> {
        let slot = self.free_slot_bits.trailing_zeros();
        let slot_mask = 1u128 << slot;
        self.free_slot_bits &= !slot_mask;
        match side {
            Side::Bid => {
                self.is_bid_bits |= slot_mask;
            }
            Side::Ask => {
                self.is_bid_bits &= !slot_mask;
            }
        };
        self.orders[slot as usize] = id;
        Ok(slot as u8)
    }
}

pub enum Market<'a> {
    V1(RefMut<'a, MarketState>),
    V2(RefMut<'a, MarketStateV2>),
}
#[derive(Copy, Clone)]
#[cfg_attr(target_endian = "little", derive(Debug))]
#[repr(packed)]
pub struct MarketStateV2 {
    pub inner: MarketState,
    pub open_orders_authority: Pubkey,
    pub prune_authority: Pubkey,
    pub consume_events_authority: Pubkey,
    // Unused bytes for future upgrades.
    padding: [u8; 992],
}

unsafe impl Zeroable for MarketStateV2 {}

#[cfg(target_endian = "little")]
unsafe impl Pod for MarketStateV2 {}

impl MarketStateV2 {
    #[inline]
    pub fn load<'a>(
        market_account: &'a AccountInfo,
        program_id: &Pubkey,
        allow_disabled: bool,
    ) -> DexResult<RefMut<'a, Self>> {
        let mut account_data: RefMut<'a, [u8]>;
        let state: RefMut<'a, Self>;

        account_data = RefMut::map(market_account.try_borrow_mut_data().unwrap(), |data| *data);

        state = RefMut::map(account_data, |data| {
            from_bytes_mut(cast_slice_mut(
                check_account_padding(data).unwrap_or_else(|_| unreachable!()),
            ))
        });

        Ok(state)
    }
}
fn check_account_padding(data: &mut [u8]) -> DexResult<&mut [[u8; 8]]> {
    let (head, data, tail) = mut_array_refs![data, 5; ..; 7];
    Ok(try_cast_slice_mut(data).unwrap())
}

impl Deref for MarketStateV2 {
    type Target = MarketState;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MarketStateV2 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl<'a> Deref for Market<'a> {
    type Target = MarketState;

    fn deref(&self) -> &Self::Target {
        match self {
            Market::V1(v1) => v1.deref(),
            Market::V2(v2) => v2.deref(),
        }
    }
}

impl<'a> Market<'a> {
    #[inline]
    pub fn load(
        market_account: &'a AccountInfo,
        program_id: &Pubkey,
        // Allow for the market flag to be set to AccountFlag::Disabled
        allow_disabled: bool,
    ) -> DexResult<Self> {
        let flags = Market::account_flags(&market_account.try_borrow_data().unwrap()).unwrap();
        if flags.intersects(AccountFlag::Permissioned) {
            Ok(Market::V2(MarketStateV2::load(
                market_account,
                program_id,
                allow_disabled,
            )?))
        } else {
            Ok(Market::V1(MarketState::load(
                market_account,
                program_id,
                allow_disabled,
            )?))
        }
    }
    pub fn account_flags(account_data: &[u8]) -> Result<BitFlags<AccountFlag>, Error> {
        let start = ACCOUNT_HEAD_PADDING.len();
        let end = start + size_of::<AccountFlag>();

        let mut flag_bytes = [0u8; 8];
        flag_bytes.copy_from_slice(&account_data[start..end]);

        BitFlags::from_bits(u64::from_le_bytes(flag_bytes))
            .map_err(|_| anyhow!("Serum error"))
            .map(Into::into)
    }
    pub fn load_orders_mut(
        &self,
        orders_account: &'a AccountInfo,
        owner_account: Option<&AccountInfo>,
        program_id: &Pubkey,
        rent: Option<Rent>,
        open_orders_authority: Option<account_parser::SignerAccount>,
    ) -> DexResult<RefMut<'a, OpenOrders>> {
        let mut open_orders: RefMut<'a, OpenOrders>;

        let open_orders_data_len = orders_account.data_len();
        let open_orders_lamports = orders_account.lamports();
        let (_, data) = strip_header::<[u8; 0], u8>(orders_account, true)?;
        open_orders = RefMut::map(data, |data| from_bytes_mut(data));

        if open_orders.account_flags == 0 {
            let oo_authority = open_orders_authority.map(|a| a.inner().key);
            if oo_authority != self.open_orders_authority() {
                return Err(DexErrorCode::InvalidOpenOrdersAuthority.into());
            }

            let rent = rent.ok_or(DexErrorCode::RentNotProvided).unwrap();
            let owner_account = owner_account
                .ok_or(DexErrorCode::OwnerAccountNotProvided)
                .unwrap();
            if !rent.is_exempt(open_orders_lamports, open_orders_data_len) {
                return Err(DexErrorCode::OrdersNotRentExempt).unwrap();
            }
            open_orders.init(
                &identity(self.own_address),
                &owner_account.key.to_aligned_bytes(),
            )?;
        }

        Ok(open_orders)
    }
    pub fn open_orders_authority(&self) -> Option<&Pubkey> {
        match &self {
            Market::V1(_) => None,
            Market::V2(state) => Some(&state.open_orders_authority),
        }
    }
}
pub fn strip_header<'a, H: Pod, D: Pod>(
    account: &'a AccountInfo,
    init_allowed: bool,
) -> DexResult<(RefMut<'a, H>, RefMut<'a, [D]>)> {
    let mut result = Ok(());
    let (header, inner): (RefMut<'a, [H]>, RefMut<'a, [D]>) =
        RefMut::map_split(account.try_borrow_mut_data().unwrap(), |padded_data| {
            let dummy_value: (&mut [H], &mut [D]) = (&mut [], &mut []);
            let padded_data: &mut [u8] = *padded_data;
            let u64_data = match strip_account_padding(padded_data, init_allowed) {
                Ok(u64_data) => u64_data,
                Err(e) => {
                    result = Err(e);
                    return dummy_value;
                }
            };

            let data: &mut [u8] = cast_slice_mut(u64_data);
            let (header_bytes, inner_bytes) = data.split_at_mut(size_of::<H>());
            let header: &mut H;
            let inner: &mut [D];

            header = match try_from_bytes_mut(header_bytes) {
                Ok(h) => h,
                Err(_e) => {
                    return dummy_value;
                }
            };
            inner = remove_slop_mut(inner_bytes);

            (std::slice::from_mut(header), inner)
        });
    result?;
    let header = RefMut::map(header, |s| s.first_mut().unwrap_or_else(|| unreachable!()));
    Ok((header, inner))
}
pub(crate) mod account_parser {
    use super::*;

    macro_rules! declare_validated_account_wrapper {
        ($WrapperT:ident, $validate:expr $(, $a:ident : $t:ty)*) => {
            #[derive(Copy, Clone)]
            pub struct $WrapperT<'a, 'b: 'a>(&'a AccountInfo<'b>);
            impl<'a, 'b: 'a> $WrapperT<'a, 'b> {
                pub fn new(account: &'a AccountInfo<'b> $(,$a: $t)*) -> DexResult<Self> {
                    let validate_result: DexResult = $validate(account $(,$a)*);
                    validate_result?;
                    Ok($WrapperT(account))
                }

                #[inline(always)]
                #[allow(unused)]
                pub fn inner(self) -> &'a AccountInfo<'b> {
                    self.0
                }
            }
        }
    }
    declare_validated_account_wrapper!(SignerAccount, |account: &AccountInfo| { Ok(()) });
}
fn strip_account_padding(padded_data: &mut [u8], init_allowed: bool) -> DexResult<&mut [[u8; 8]]> {
    if init_allowed {
        init_account_padding(padded_data)
    } else {
        check_account_padding(padded_data)
    }
}
fn init_account_padding(data: &mut [u8]) -> DexResult<&mut [[u8; 8]]> {
    let (head, data, tail) = mut_array_refs![data, 5; ..; 7];
    *head = *ACCOUNT_HEAD_PADDING;
    *tail = *ACCOUNT_TAIL_PADDING;
    Ok(try_cast_slice_mut(data).unwrap())
}
#[inline]
fn remove_slop<T: Pod>(bytes: &[u8]) -> &[T] {
    let slop = bytes.len() % size_of::<T>();
    let new_len = bytes.len() - slop;
    cast_slice(&bytes[..new_len])
}

#[inline]
fn remove_slop_mut<T: Pod>(bytes: &mut [u8]) -> &mut [T] {
    let slop = bytes.len() % size_of::<T>();
    let new_len = bytes.len() - slop;
    cast_slice_mut(&mut bytes[..new_len])
}
pub trait ToAlignedBytes {
    fn to_aligned_bytes(&self) -> [u64; 4];
}

impl ToAlignedBytes for Pubkey {
    #[inline]
    fn to_aligned_bytes(&self) -> [u64; 4] {
        cast(self.to_bytes())
    }
}
#[derive(Debug)]
pub enum EventView {
    Fill {
        side: Side,
        maker: bool,
        native_qty_paid: u64,
        native_qty_received: u64,
        native_fee_or_rebate: u64,
        order_id: u128,
        owner: [u64; 4],
        owner_slot: u8,
        fee_tier: FeeTier,
        client_order_id: Option<NonZeroU64>,
    },
    Out {
        side: Side,
        release_funds: bool,
        native_qty_unlocked: u64,
        native_qty_still_locked: u64,
        order_id: u128,
        owner: [u64; 4],
        owner_slot: u8,
        client_order_id: Option<NonZeroU64>,
    },
}

impl EventView {
    fn side(&self) -> Side {
        match self {
            EventView::Fill { side, .. } => *side,
            &EventView::Out { side, .. } => side,
        }
    }
}
#[derive(Copy, Clone, IntoPrimitive, TryFromPrimitive, Debug)]
#[repr(u8)]
pub enum FeeTier {
    Base,
    _SRM2,
    _SRM3,
    _SRM4,
    _SRM5,
    _SRM6,
    _MSRM,
    Stable,
}
