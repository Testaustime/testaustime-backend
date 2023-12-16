use chrono::{prelude::*, Duration};
use diesel::prelude::*;

use crate::{
    error::TimeError,
    models::*,
    requests::{DataRequest, HeartBeat},
};

impl super::DatabaseWrapper {
    pub async fn add_activity(
        &self,
        updated_user_id: i32,
        heartbeat: HeartBeat,
        ctx_start_time: NaiveDateTime,
        ctx_duration: Duration,
    ) -> Result<(), TimeError> {
        let activity = NewCodingActivity {
            user_id: updated_user_id,
            start_time: ctx_start_time,
            duration: ctx_duration.num_seconds() as i32,
            // FIXME: wtf why is this done in the database?
            project_name: if heartbeat.project_name.is_some()
                && heartbeat.project_name.as_ref().unwrap().starts_with("tmp.")
            {
                Some(String::from("tmp"))
            } else {
                heartbeat.project_name.map(|s| s.to_lowercase())
            },
            language: heartbeat.language,
            editor_name: heartbeat.editor_name,
            hostname: heartbeat.hostname,
        };

        self.run_async_query(move |mut conn| {
            use crate::schema::coding_activities::dsl::*;

            diesel::insert_into(coding_activities)
                .values(activity)
                .execute(&mut conn)?;

            Ok(())
        })
        .await?;

        Ok(())
    }

    pub async fn get_all_activity(&self, user: i32) -> Result<Vec<CodingActivity>, TimeError> {
        self.run_async_query(move |mut conn| {
            use crate::schema::coding_activities::dsl::*;
            Ok(coding_activities
                .filter(user_id.eq(user))
                .load::<CodingActivity>(&mut conn)?)
        })
        .await
    }

    pub async fn get_activity(
        &self,
        request: DataRequest,
        user: i32,
    ) -> Result<Vec<CodingActivity>, TimeError> {
        use crate::schema::coding_activities::dsl::*;
        let mut query = coding_activities.into_boxed().filter(user_id.eq(user));
        if let Some(from) = request.from {
            query = query.filter(start_time.ge(from.naive_local()));
        };
        if let Some(to) = request.to {
            query = query.filter(start_time.le(to.naive_local()));
        };
        if let Some(editor) = request.editor_name {
            query = query.filter(editor_name.eq(editor));
        };
        if let Some(project) = request.project_name {
            query = query.filter(project_name.eq(project));
        };
        if let Some(request_hostname) = request.hostname {
            query = query.filter(hostname.eq(request_hostname));
        };
        if let Some(request_language) = request.language {
            query = query.filter(language.eq(request_language));
        };
        if let Some(min_duration) = request.min_duration {
            query = query.filter(duration.ge(min_duration));
        };

        self.run_async_query(move |mut conn| Ok(query.load::<CodingActivity>(&mut conn)?))
            .await
    }

    pub async fn get_user_coding_time_since(
        &self,
        uid: i32,
        since: chrono::NaiveDateTime,
    ) -> Result<i32, TimeError> {
        self.run_async_query(move |mut conn| {
            use crate::schema::coding_activities::dsl::*;

            Ok(coding_activities
                .filter(user_id.eq(uid).and(start_time.ge(since)))
                .select(diesel::dsl::sum(duration))
                .first::<Option<i64>>(&mut conn)?
                .unwrap_or(0) as i32)
        })
        .await
    }

    pub async fn get_coding_time_steps(&self, uid: i32) -> CodingTimeSteps {
        CodingTimeSteps {
            all_time: self
                .get_user_coding_time_since(uid, chrono::NaiveDateTime::from_timestamp(0, 0))
                .await
                .unwrap_or(0),
            past_month: self
                .get_user_coding_time_since(
                    uid,
                    chrono::Local::now().naive_local() - chrono::Duration::days(30),
                )
                .await
                .unwrap_or(0),
            past_week: self
                .get_user_coding_time_since(
                    uid,
                    chrono::Local::now().naive_local() - chrono::Duration::days(7),
                )
                .await
                .unwrap_or(0),
        }
    }

    pub async fn rename_project(
        &self,
        target_user_id: i32,
        from: String,
        to: String,
    ) -> Result<usize, TimeError> {
        self.run_async_query(move |mut conn| {
            use crate::schema::coding_activities::dsl::*;
            Ok(diesel::update(coding_activities)
                .filter(user_id.eq(target_user_id))
                .filter(project_name.eq(from))
                .set(project_name.eq(to))
                .execute(&mut conn)?)
        })
        .await
    }

    pub async fn delete_activity(&self, userid: i32, activity: i32) -> Result<bool, TimeError> {
        use crate::schema::coding_activities::dsl::*;

        let res = self
            .run_async_query(move |mut conn| {
                Ok(diesel::delete(coding_activities.find(activity))
                    // FIXME: This filter is useless?
                    .filter(user_id.eq(userid))
                    .execute(&mut conn)?)
            })
            .await?;

        Ok(res != 0)
    }
}
