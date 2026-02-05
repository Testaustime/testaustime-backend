use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Algorithm, Argon2, Params, PasswordHash, PasswordVerifier, Version,
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

    pub async fn get_user_by_name(&self, target_username: &str) -> Result<UserIdentity, TimeError> {
        let mut conn = self.db.get().await?;
        use crate::schema::user_identities::dsl::*;
        define_sql_function!(fn lower(x: diesel::sql_types::Text) -> Text);

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

    pub async fn verify_user_password(
        &self,
        arg_username: &str,
        password: &str,
    ) -> Result<Option<UserIdentity>, TimeError> {
        let mut conn = self.db.get().await?;
        define_sql_function!(fn lower(x: diesel::sql_types::Text) -> Text);

        use user_identities::dsl::username;

        let (user, tuser) = user_identities::table
            .filter(lower(username).eq(arg_username.to_lowercase()))
            .inner_join(testaustime_users::table)
            .first::<(UserIdentity, TestaustimeUser)>(&mut conn)
            .await?;

        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(4096, 3, 1, None).expect("BUG: Hardcoded params, wont change"),
        );

        let hash = PasswordHash::new(&tuser.password)
            .expect("BUG: This is valid if it was succesfully inserted into database");

        if argon2.verify_password(password.as_bytes(), &hash).is_ok() {
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    pub async fn regenerate_token(&self, userid: i32) -> Result<String, TimeError> {
        let mut conn = self.db.get().await?;

        let token = crate::utils::generate_auth_token();

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
        email: Option<&str>,
    ) -> Result<NewUserIdentity, TimeError> {
        if self.user_exists(username.to_string()).await? {
            return Err(TimeError::UsernameTaken);
        }
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(4096, 3, 1, None).expect("BUG: Hardcoded params, wont change"),
        );

        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .expect("BUG: Hashing failed on sanitized output");
        let token = generate_auth_token();
        let new_user = NewUserIdentity {
            auth_token: token,
            registration_time: chrono::Local::now().naive_local(),
            username: username.to_string(),
            friend_code: generate_friend_code(),
            email: email.map(String::from),
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
                        .map_err(|_| TimeError::UsernameTaken)?;

                    let testaustime_user = NewTestaustimeUser {
                        password: hash.to_string(),
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

    pub async fn change_username(&self, user: i32, new_username: &str) -> Result<(), TimeError> {
        let mut conn = self.db.get().await?;

        use crate::schema::user_identities::dsl::*;
        diesel::update(crate::schema::user_identities::table)
            .filter(id.eq(user))
            .set(username.eq(new_username))
            .execute(&mut conn)
            .await
            .map_err(|_| TimeError::UsernameTaken)?;

        Ok(())
    }

    pub async fn change_email(&self, user: i32, new_email: String) -> Result<(), TimeError> {
        let mut conn = self.db.get().await?;

        use crate::schema::user_identities::dsl::*;
        diesel::update(crate::schema::user_identities::table)
            .filter(id.eq(user))
            .set(email.eq(new_email))
            .execute(&mut conn)
            .await
            .map_err(|_| TimeError::EmailTaken)?;

        Ok(())
    }

    pub async fn change_password(&self, user: i32, new_password: &str) -> Result<(), TimeError> {
        let new_salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(4096, 3, 1, None).expect("BUG: Hardcoded params, wont change"),
        );

        let hash = argon2
            .hash_password(new_password.as_bytes(), &new_salt)
            .expect("BUG: Hashing failed on sanitized output");

        let mut conn = self.db.get().await?;

        use crate::schema::testaustime_users::dsl::*;
        diesel::update(crate::schema::testaustime_users::table)
            .filter(identity.eq(user))
            .set((password.eq(&hash.to_string()),))
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

    pub async fn get_user_by_email(
        &self,
        searched_email: &str,
    ) -> Result<Option<UserIdentity>, TimeError> {
        let mut conn = self.db.get().await?;
        let user = {
            use crate::schema::user_identities::dsl::*;

            user_identities
                .filter(email.eq(searched_email))
                .first::<UserIdentity>(&mut conn)
                .await
                .optional()?
        };

        Ok(user)
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
                auth_token: generate_auth_token(),
                registration_time: chrono::Local::now().naive_local(),
                username,
                friend_code: generate_friend_code(),
                email: None,
            };
            let new_user_id = diesel::insert_into(crate::schema::user_identities::table)
                .values(&new_user)
                .returning(id)
                .get_results::<i32>(&mut conn)
                .await
                .map_err(|_| TimeError::UsernameTaken)?;

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
