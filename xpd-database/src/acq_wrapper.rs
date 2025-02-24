use std::ops::DerefMut;

use futures_core::future::BoxFuture;

use crate::Error;

/// This trait wraps [`sqlx::Acquire`] to return errors compatible with [`crate::Error`]
pub trait AcquireWrapper<'c> {
    type Database: sqlx::Database;
    type Connection: std::ops::Deref<Target = <Self::Database as sqlx::Database>::Connection>
        + DerefMut
        + Send;

    /// This function is so named to not collide with the associated method [`sqlx::Pool::begin`].
    /// It is equivalent except for automatically wrapping the error.
    async fn xbegin(self) -> Result<TransactionWrapper<'c, Self::Database>, Error>;
}

impl<'c, DB: sqlx::Database> AcquireWrapper<'c> for &'c sqlx::Pool<DB> {
    type Connection = <&'c sqlx::Pool<DB> as sqlx::Acquire<'c>>::Connection;
    type Database = <&'c sqlx::Pool<DB> as sqlx::Acquire<'c>>::Database;

    async fn xbegin(self) -> Result<TransactionWrapper<'c, Self::Database>, Error> {
        match self.begin().await {
            Ok(v) => Ok(TransactionWrapper(v)),
            Err(e) => Err(Error::Database(e)),
        }
    }
}

/// This struct wraps [`sqlx::Transaction`] to return errors compatible with [`crate::Error`].
/// It is mostly equivalent, though not as comprehensive. All implementations forward directly
/// to [`sqlx`] for non-error-handling behaviors.
pub struct TransactionWrapper<'a, DB: sqlx::Database>(sqlx::Transaction<'a, DB>);

impl<DB: sqlx::Database> TransactionWrapper<'_, DB> {
    pub async fn commit(self) -> Result<(), Error> {
        Ok(self.0.commit().await?)
    }

    pub async fn rollback(self) -> Result<(), Error> {
        Ok(self.0.rollback().await?)
    }
}

impl<'t, DB: sqlx::Database> sqlx::Acquire<'t> for &'t mut TransactionWrapper<'_, DB> {
    type Connection = &'t mut <DB as sqlx::Database>::Connection;
    type Database = DB;

    #[inline]
    fn acquire(self) -> BoxFuture<'t, Result<Self::Connection, sqlx::Error>> {
        self.0.acquire()
    }

    #[inline]
    fn begin(self) -> BoxFuture<'t, Result<sqlx::Transaction<'t, DB>, sqlx::Error>> {
        self.0.begin()
    }
}

impl<DB: sqlx::Database> AsMut<DB::Connection> for TransactionWrapper<'_, DB> {
    fn as_mut(&mut self) -> &mut DB::Connection {
        &mut self.0
    }
}
