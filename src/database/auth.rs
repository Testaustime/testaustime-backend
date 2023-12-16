use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use diesel::prelude::*;

use crate::{error::TimeError, models::*, utils::*};

impl super::DatabaseWrapper {
    pub async fn user_exists(&self, target_username: String) -> Result<bool, TimeError> {
        use crate::schema::user_identities::dsl::*;

        self.run_async_query(move |mut conn| {
            Ok(user_identities
                .filter(username.eq(target_username))
                .first::<UserIdentity>(&mut conn)
                .optional()?
                .is_some())
        })
        .await
    }

    pub async fn get_user_by_name(
        &self,
        target_username: String,
    ) -> Result<UserIdentity, TimeError> {
        use crate::schema::user_identities::dsl::*;
        sql_function!(fn lower(x: diesel::sql_types::Text) -> Text);

        self.run_async_query(move |mut conn| {
            Ok(user_identities
                .filter(lower(username).eq(target_username.to_lowercase()))
                .first::<UserIdentity>(&mut conn)?)
        })
        .await
    }

    pub async fn delete_user(&self, userid: i32) -> Result<bool, TimeError> {
        use crate::schema::user_identities::dsl::*;

        self.run_async_query(move |mut conn| {
            Ok(diesel::delete(user_identities.find(userid)).execute(&mut conn)? > 0)
        })
        .await
    }

    pub async fn get_user_by_id(&self, userid: i32) -> Result<UserIdentity, TimeError> {
        use crate::schema::user_identities::dsl::*;

        self.run_async_query(move |mut conn| {
            Ok(user_identities
                .find(userid)
                .first::<UserIdentity>(&mut conn)?)
        })
        .await
    }

    // TODO: get rid of unwraps
    pub async fn verify_user_password(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<UserIdentity>, TimeError> {
        let user = self.get_user_by_name(username.to_string()).await?;
        let tuser = self.get_testaustime_user_by_id(user.id).await?;

        let argon2 = Argon2::default();
        let Ok(salt) = SaltString::new(std::str::from_utf8(&tuser.salt).unwrap()) else {
            return Ok(None); // The user has no password
        };
        let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();
        if password_hash.hash.unwrap().as_bytes() == tuser.password {
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    pub async fn regenerate_token(&self, userid: i32) -> Result<String, TimeError> {
        let token = crate::utils::generate_token();

        let token_clone = token.clone();

        self.run_async_query(move |mut conn| {
            use crate::schema::user_identities::dsl::*;

            diesel::update(user_identities.find(userid))
                .set(auth_token.eq(token_clone))
                .execute(&mut conn)?;

            Ok(())
        })
        .await?;

        Ok(token)
    }

    pub async fn new_testaustime_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<NewUserIdentity, TimeError> {
        use crate::schema::{testaustime_users, user_identities};
        if self.user_exists(username.to_string()).await? {
            return Err(TimeError::UserExists);
        }
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();
        let token = generate_token();
        let hash = password_hash.hash.unwrap();
        let new_user = NewUserIdentity {
            auth_token: token,
            registration_time: chrono::Local::now().naive_local(),
            username: username.to_string(),
            friend_code: generate_friend_code(),
        };

        let new_user_clone = new_user.clone();

        self.run_async_query(move |mut conn| {
            let id = diesel::insert_into(crate::schema::user_identities::table)
                .values(new_user_clone)
                .returning(user_identities::id)
                .get_results::<i32>(&mut conn)
                .map_err(|_| TimeError::UserExists)?;

            let testaustime_user = NewTestaustimeUser {
                password: hash.as_bytes().to_vec(),
                salt: salt.as_bytes().to_vec(),
                identity: id[0],
            };

            diesel::insert_into(testaustime_users::table)
                .values(&testaustime_user)
                .execute(&mut conn)?;

            Ok(())
        })
        .await?;

        Ok(new_user)
    }

    pub async fn change_username(&self, user: i32, new_username: String) -> Result<(), TimeError> {
        if self.user_exists(new_username.to_string()).await? {
            return Err(TimeError::UserExists);
        }

        self.run_async_query(move |mut conn| {
            use crate::schema::user_identities::dsl::*;
            diesel::update(crate::schema::user_identities::table)
                .filter(id.eq(user))
                .set(username.eq(new_username))
                .execute(&mut conn)
                .map_err(|_| TimeError::UserExists)?;
            Ok(())
        })
        .await?;

        Ok(())
    }

    pub async fn change_password(&self, user: i32, new_password: &str) -> Result<(), TimeError> {
        let new_salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(new_password.as_bytes(), &new_salt)
            .unwrap();
        let new_hash = password_hash.hash.unwrap();

        self.run_async_query(move |mut conn| {
            use crate::schema::testaustime_users::dsl::*;
            diesel::update(crate::schema::testaustime_users::table)
                .filter(identity.eq(user))
                .set((
                    password.eq(&new_hash.as_bytes()),
                    salt.eq(new_salt.as_bytes()),
                ))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?;

        Ok(())
    }

    pub async fn get_user_by_token(&self, token: String) -> Result<UserIdentity, TimeError> {
        let user = self
            .run_async_query(move |mut conn| {
                use crate::schema::user_identities::dsl::*;

                Ok(user_identities
                    .filter(auth_token.eq(token))
                    .first::<UserIdentity>(&mut conn)?)
            })
            .await?;

        Ok(user)
    }

    pub async fn get_testaustime_user_by_id(&self, uid: i32) -> Result<TestaustimeUser, TimeError> {
        self.run_async_query(move |mut conn| {
            use crate::schema::testaustime_users::dsl::*;

            Ok(testaustime_users
                .filter(identity.eq(uid))
                .first::<TestaustimeUser>(&mut conn)?)
        })
        .await
    }

    #[cfg(feature = "testausid")]
    pub async fn testausid_login(
        &self,
        user_id_arg: String,
        username: String,
        platform_id: String,
    ) -> Result<String, TimeError> {
        use crate::schema::{
            testausid_users::dsl::{identity, testausid_users, user_id},
            user_identities::dsl::{auth_token, id, user_identities},
        };

        let user_id_arg_clone = user_id_arg.clone();

        let user_identity_opt = self
            .run_async_query(move |mut conn| {
                Ok(testausid_users
                    .filter(user_id.eq(user_id_arg_clone))
                    .select(identity)
                    .first::<i32>(&mut conn)
                    .optional()?)
            })
            .await?;

        if let Some(user_identity) = user_identity_opt {
            let token = self
                .run_async_query(move |mut conn| {
                    Ok(user_identities
                        .find(user_identity)
                        .select(auth_token)
                        .first::<String>(&mut conn)?)
                })
                .await?;

            Ok(token)
        } else {
            let token = generate_token();
            let new_user = NewUserIdentity {
                //FIXME: You can get around using a clone here
                auth_token: token.clone(),
                registration_time: chrono::Local::now().naive_local(),
                username,
                friend_code: generate_friend_code(),
            };
            let new_user_id = self
                .run_async_query(move |mut conn| {
                    diesel::insert_into(crate::schema::user_identities::table)
                        .values(&new_user)
                        .returning(id)
                        .get_results::<i32>(&mut conn)
                        .map_err(|_| TimeError::UserExists)
                })
                .await?;

            let testausid_user = NewTestausIdUser {
                user_id: user_id_arg,
                identity: new_user_id[0],
                service_id: platform_id,
            };

            self.run_async_query(move |mut conn| {
                diesel::insert_into(testausid_users)
                    .values(&testausid_user)
                    .execute(&mut conn)?;

                Ok(())
            })
            .await?;

            Ok(token)
        }
    }

    pub async fn change_visibility(&self, userid: i32, visibility: bool) -> Result<(), TimeError> {
        self.run_async_query(move |mut conn| {
            use crate::schema::user_identities::dsl::*;
            diesel::update(user_identities.find(userid))
                .set(is_public.eq(visibility))
                .execute(&mut conn)?;
            Ok(())
        })
        .await
    }
}
