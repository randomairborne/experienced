use twilight_model::id::Id;

pub trait ReinterpretPrimitiveBits<O> {
    fn reinterpret_bits(&self) -> O;
}

macro_rules! impl_primitive_reinterpret {
    ($from:ty, $to:ty) => {
        impl ReinterpretPrimitiveBits<$to> for $from {
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
            fn reinterpret_bits(&self) -> $to {
                *self as $to
            }
        }
    };
}

impl_primitive_reinterpret!(u8, i8);
impl_primitive_reinterpret!(u16, i16);
impl_primitive_reinterpret!(u32, i32);
impl_primitive_reinterpret!(u64, i64);
impl_primitive_reinterpret!(u128, i128);
impl_primitive_reinterpret!(i8, u8);
impl_primitive_reinterpret!(i16, u16);
impl_primitive_reinterpret!(i32, u32);
impl_primitive_reinterpret!(i64, u64);
impl_primitive_reinterpret!(i128, u128);

/// Fetches the raw ID data from Twilight and returns it as an i64, so it can be stored in Postgres
/// easily.
/// Essentially a no-op.
#[must_use]
#[inline]
pub fn id_to_db<T>(id: Id<T>) -> i64 {
    id.get().reinterpret_bits()
}

/// Create a new checked twilight id from an i64. Only get this from the DB!
/// Essentially a no-op.
#[inline]
#[must_use]
pub fn db_to_id<T>(db: i64) -> Id<T> {
    Id::new(db.reinterpret_bits())
}
