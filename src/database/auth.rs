use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{
    error::TimeError,
    models::*,
    schema::{testaustime_users, user_identities},
    utils::*,
};

impl super::DatabaseWrapper {
    pub async fn user_exists(&self, target_username: String) -> Result<bool, TimeError> {
        let mut conn = self.db.get().await?;
        use crate::schema::user_identities::dsl::*;

        Ok(user_identities
            .filter(username.eq(target_username))
            .first::<UserIdentity>(&mut conn)
            .await
            .optional()?
            .is_some())
    }

    pub async fn get_user_by_name(
        &self,
        target_username: String,
    ) -> Result<UserIdentity, TimeError> {
        let mut conn = self.db.get().await?;
        use crate::schema::user_identities::dsl::*;
        sql_function!(fn lower(x: diesel::sql_types::Text) -> Text);

        Ok(user_identities
            .filter(lower(username).eq(target_username.to_lowercase()))
            .first::<UserIdentity>(&mut conn)
            .await?)
    }

    pub async fn delete_user(&self, userid: i32) -> Result<bool, TimeError> {
        let mut conn = self.db.get().await?;
        use crate::schema::user_identities::dsl::*;

        Ok(diesel::delete(user_identities.find(userid))
            .execute(&mut conn)
            .await?
            > 0)
    }

    pub async fn get_user_by_id(&self, userid: i32) -> Result<UserIdentity, TimeError> {
        let mut conn = self.db.get().await?;
        use crate::schema::user_identities::dsl::*;

        Ok(user_identities
            .find(userid)
            .first::<UserIdentity>(&mut conn)
            .await?)
    }

    // TODO: get rid of unwraps
    pub async fn verify_user_password(
        &self,
        arg_username: &str,
        password: &str,
    ) -> Result<Option<UserIdentity>, TimeError> {
        let mut conn = self.db.get().await?;

        use user_identities::dsl::username;

        let (user, tuser) = user_identities::table
            .filter(username.eq(arg_username))
            .inner_join(testaustime_users::table)
            .first::<(UserIdentity, TestaustimeUser)>(&mut conn)
            .await?;

        let argon2 = Argon2::default();
        let Ok(salt) = SaltString::new(std::str::from_utf8(&tuser.salt).expect("bug: impossible"))
        else {
            return Ok(None); // The user has no password
        };
        let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();
        if password_hash.hash.expect("bug: impossible").as_bytes() == tuser.password {
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    pub async fn regenerate_token(&self, userid: i32) -> Result<String, TimeError> {
        let mut conn = self.db.get().await?;

        let token = crate::utils::generate_token();

        use crate::schema::user_identities::dsl::*;

        diesel::update(user_identities.find(userid))
            .set(auth_token.eq(&token))
            .execute(&mut conn)
            .await?;

        Ok(token)
    }

    pub async fn new_testaustime_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<NewUserIdentity, TimeError> {
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

        let mut conn = self.db.get().await?;

        conn.build_transaction()
            .read_write()
            .deferrable()
            .run(|mut conn| {
                Box::pin(async move {
                    let id = diesel::insert_into(crate::schema::user_identities::table)
                        .values(new_user_clone)
                        .returning(user_identities::id)
                        .get_results::<i32>(&mut conn)
                        .await
                        .map_err(|_| TimeError::UserExists)?;

                    let testaustime_user = NewTestaustimeUser {
                        password: hash.as_bytes().to_vec(),
                        salt: salt.as_bytes().to_vec(),
                        identity: id[0],
                    };

                    diesel::insert_into(testaustime_users::table)
                        .values(&testaustime_user)
                        .execute(&mut conn)
                        .await?;

                    Ok::<(), TimeError>(())
                }) as _
            })
            .await?;

        Ok(new_user)
    }

    pub async fn change_username(&self, user: i32, new_username: String) -> Result<(), TimeError> {
        let mut conn = self.db.get().await?;

        conn.build_transaction()
            .read_write()
            .run(|mut conn| {
                Box::pin(async move {
                    use crate::schema::user_identities::dsl::*;

                    if (user_identities
                        .filter(username.eq(new_username.clone()))
                        .first::<UserIdentity>(&mut conn)
                        .await)
                        .is_ok()
                    {
                        return Err(TimeError::UserExists);
                    };

                    diesel::update(crate::schema::user_identities::table)
                        .filter(id.eq(user))
                        .set(username.eq(new_username))
                        .execute(&mut conn)
                        .await
                        .map_err(|_| TimeError::UserExists)?;

                    Ok::<(), TimeError>(())
                })
            })
            .await
    }

    pub async fn change_password(&self, user: i32, new_password: &str) -> Result<(), TimeError> {
        let new_salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(new_password.as_bytes(), &new_salt)
            .unwrap();
        let new_hash = password_hash.hash.unwrap();

        let mut conn = self.db.get().await?;

        use crate::schema::testaustime_users::dsl::*;
        diesel::update(crate::schema::testaustime_users::table)
            .filter(identity.eq(user))
            .set((
                password.eq(&new_hash.as_bytes()),
                salt.eq(new_salt.as_bytes()),
            ))
            .execute(&mut conn)
            .await?;

        Ok(())
    }

    pub async fn get_user_by_token(&self, token: String) -> Result<UserIdentity, TimeError> {
        let mut conn = self.db.get().await?;
        let user = {
            use crate::schema::user_identities::dsl::*;

            user_identities
                .filter(auth_token.eq(token))
                .first::<UserIdentity>(&mut conn)
                .await?
        };

        Ok(user)
    }

    pub async fn get_testaustime_user_by_id(&self, uid: i32) -> Result<TestaustimeUser, TimeError> {
        use crate::schema::testaustime_users::dsl::*;
        let mut conn = self.db.get().await?;

        Ok(testaustime_users
            .filter(identity.eq(uid))
            .first::<TestaustimeUser>(&mut conn)
            .await?)
    }

    // FIXME: Use transactions
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

        let mut conn = self.db.get().await?;

        let user_identity_opt = testausid_users
            .filter(user_id.eq(&user_id_arg))
            .select(identity)
            .first::<i32>(&mut conn)
            .await
            .optional()?;

        if let Some(user_identity) = user_identity_opt {
            let token = user_identities
                .find(user_identity)
                .select(auth_token)
                .first::<String>(&mut conn)
                .await?;

            Ok(token)
        } else {
            let new_user = NewUserIdentity {
                auth_token: generate_token(),
                registration_time: chrono::Local::now().naive_local(),
                username,
                friend_code: generate_friend_code(),
            };
            let new_user_id = diesel::insert_into(crate::schema::user_identities::table)
                .values(&new_user)
                .returning(id)
                .get_results::<i32>(&mut conn)
                .await
                .map_err(|_| TimeError::UserExists)?;

            let testausid_user = NewTestausIdUser {
                user_id: user_id_arg,
                identity: new_user_id[0],
                service_id: platform_id,
            };

            diesel::insert_into(testausid_users)
                .values(&testausid_user)
                .execute(&mut conn)
                .await?;

            Ok(new_user.auth_token)
        }
    }

    pub async fn change_visibility(&self, userid: i32, visibility: bool) -> Result<(), TimeError> {
        let mut conn = self.db.get().await?;

        use crate::schema::user_identities::dsl::*;
        diesel::update(user_identities.find(userid))
            .set(is_public.eq(visibility))
            .execute(&mut conn)
            .await?;
        Ok(())
    }
}
