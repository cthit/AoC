use rocket_sync_db_pools::database;

#[database("redis")]
pub struct RedisConn(redis_pool::Redis);

mod redis_pool {
	use std::ops::{Deref, DerefMut};

	use r2d2_redis::{
		r2d2::ManageConnection,
		redis::{Connection, RedisError},
		RedisConnectionManager,
	};
	use rocket::{Build, Rocket};
	use rocket_sync_db_pools::{r2d2, Config, Error, PoolResult, Poolable};

	pub struct Redis(Connection);
	pub struct RedisManager(RedisConnectionManager);

	impl Deref for Redis {
		type Target = Connection;

		fn deref(&self) -> &Self::Target {
			&self.0
		}
	}

	impl DerefMut for Redis {
		fn deref_mut(&mut self) -> &mut Self::Target {
			&mut self.0
		}
	}

	impl ManageConnection for RedisManager {
		type Connection = Redis;
		type Error = r2d2_redis::Error;

		fn connect(&self) -> Result<Self::Connection, Self::Error> {
			self.0.connect().map(Redis)
		}

		fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
			self.0.is_valid(&mut conn.0)
		}

		fn has_broken(&self, conn: &mut Self::Connection) -> bool {
			self.0.has_broken(&mut conn.0)
		}
	}

	impl Poolable for Redis {
		type Error = RedisError;
		type Manager = RedisManager;

		fn pool(db_name: &str, rocket: &Rocket<Build>) -> PoolResult<Self> {
			let config = Config::from(db_name, rocket)?;
			let manager =
				RedisManager(RedisConnectionManager::new(config.url).map_err(Error::Custom)?);
			Ok(r2d2::Pool::builder()
				.max_size(config.pool_size)
				.build(manager)?)
		}
	}
}
