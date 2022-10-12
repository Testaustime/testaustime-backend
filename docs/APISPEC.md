# Testaustime-rs api documentation

## General info

Testaustime API gives 5 different routes:
- [/auth/](#auth)
- [/users/](#users)
- [/activity/](#activity)
- [/friends/](#friends)
- [/leaderboards/](#leaderboards)

Basic path: `https://api.testaustime.fi`

Limits:
- Usual Ratelimit: 10 req/m.

## <a name="auth"></a>  Auth

Contains various user authorization operations

### Endpoints

| Endpoint|  Method | Description |
| --- | --- | --- |
| [/auth/register](#register) | POST | Creating a new user and returns the user auth token, friend code and registration time |
| [/auth/login](#login) | POST | Loging user to system and returns the user auth token and friend code |
| [/auth/changeusername](#changeusername) | POST | Changing user username |
| [/auth/changepassword](#changepassword) | POST | Changing user password |
| [/auth/regenerate](#regenerate)  | POST | Regenerating user auth token |

#### <a name="register"></a>    [1. POST /auth/register](#auth)

Creating a new user and returns the user auth token, friend code and registration time. Ratelimit: 1 req/24h

<details>
  <summary>Header params</summary>

| Name |  Value |
| --- | --- |
| Content-Type | application/json |
</details>

<details>
  <summary>Body params</summary>

| Param |  Type | Required | Description |
| --- | --- | --- | --- |
| username | string | Yes | Usename has to be between 2 and 32 characters long |
| password | string | Yes | Password has to be between 8 and 128 characters long |
</details>

**Sample request**
```curl
curl --request POST https://api.testaustime.fi/auth/register' \
--header 'Content-Type: application/json' \
--data-raw '{
    "username": "username",
    "password": "password"
}
```
**Sample response**
```JSON
{
    "auth_token": "<token>",
    "username": "username",
    "friend_code": "friend_code",
    "registration_time": "YYYY-MM-DDTHH:MM:SS.sssssssssZ"
}
```
<details>
  <summary>Response definitions</summary>

| Response Item | Type | Description |
| --- | --- | --- |
| auth_token | string | Bearer Auth token. Using for the all next resquests to identify user |
| username | string | Username |
| friend_code | string | By this code another users can add user to the friend list |
| registration_time | string (ISO 8601 format) | Time of registration to nanoseconds |
</details>


#### <a name="login"></a>  [2. POST /auth/login](#auth)

Logins to a users account and returning the auth token

<details>
  <summary>Header params</summary>

  | Name |  Value |
| --- | --- |
| Content-Type | application/json |
</details>

<details>
  <summary>Body params</summary>

| Param |  Type | Required | Description |
| --- | --- | --- | --- |
| username | string | Yes | Usename has to be between 2 and 32 characters long |
| password | string | Yes | Password has to be between 8 and 128 characters long |
</details>

**Sample request**
```curl
curl --request POST 'https://api.testaustime.fi/auth/login' \
--header 'Content-Type: application/json' \
--data-raw '{
    "username": "username",
    "password": "password"
}'

```
**Sample response**
```JSON
{
    "id": 0,
    "auth_token": "<token>",
    "friend_code": "friend_code",
    "username": "username",
    "registration_time": "YYYY-MM-DDTHH:MM:SS.ssssssZ"
}
```

<details>
  <summary>Response definitions</summary>

| Response Item | Type | Description |
| --- | --- | --- |
| id | int| User id |
| auth_token | string | Bearer Auth token. Using for the all next resquests to identify user |
| friend_code | string | By this code another users can add user to the friend list |
| username | string | Username |
| registration_time | string (ISO 8601 format) | Time of registration to microsends |
</details>

#### <a name="changeusername"></a>   [3. POST /auth/changeusername](#auth)

Changes username

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Content-Type | application/json |
| Authorization | Bearer `<token>` |
</details>

<details>
  <summary>Body params:</summary>

| Param |  Type | Required | Description |
| --- | --- | --- | --- |
| new | string | Yes | New username. Usename has to be between 2 and 32 characters long |
</details>

**Sample request**
```curl
curl --request POST 'https://api.testaustime.fi/auth/changeusername' \
--header 'Content-Type: application/json' \
--header 'Authorization: Bearer <token> '{
    "new": "new_username"
}'
```

**Sample response**
```http
200 OK
```
<details>
  <summary>Error examples:</summary>

| Error | Error code | Body |
| --- | --- | --- |
| "new" has <2 or >32 symbols | 400 Bad Request | `{"error" : "Username is not between 2 and 32 chars"}` |
| "new" is using existing username| 403 Forbidden | `"error"» : "User exists"` |
</details>

#### <a name="changepassword"></a>  [4. POST /auth/changepassword](#auth)

Changes users password

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Content-Type | application/json |
| Authorization | Bearer `<token>` |
</details>

<details>
  <summary>Body params:</summary>

| Param |  Type | Required | Description |
| --- | --- | --- | --- |
| old | string | Yes | Current password |
| new | string | Yes | New password. Password has to be between 8 and 128 characters long |
</details>

**Sample request**
```curl
curl --request POST 'https://api.testaustime.fi/auth/changepassword' \
--header 'Content-Type: application/json' \
--header 'Authorization: Bearer <token>' \
--data-raw '{
   "old": "old_password",
   "new": "new_password"
}'
```

**Sample response**
```http
200 OK
```

<details>
  <summary>Error examples:</summary>

| Error | Error code | Body |
| --- | --- | --- |
| "new" has < 8 or >132 symbols  | 400 Bad Request | `{"error": "Password is not between 8 and 132 chars"}` |
| "old" is incorrect| 401 Unathorized | `{"error": "You are not authorized"}` |
</details>

#### <a name="regenerate"></a>  [5. POST /auth/regenerate](#auth)

Regenerates users auth token

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
</details>

**Sample request**
```curl
curl --request POST 'https://api.testaustime.fi/auth/regenerate' \
--header 'Content-Type: application/json'
```

**Sample response**
```JSON
{
    "token": "<token>"
}
```

<details>
  <summary>Response definitions:</summary>

| Response Item | Type | Description |
| --- | --- | --- |
| token | string| New Bearer Auth token. Using for the all next resquests to identify user |
</details>

## <a name="users"></a>  Users

Containts various mostfully read-operations with user data

### Endpoints

| Endpoint                                                | Method | Description                                     |
| ---                                                     | ---    | ---                                             |
| [/users/@me](#me)                                       | GET    | Geting data about authorized user               |
| [/users/@me/leaderboards](#my_leaderboards)             | GET    | Geting list of user leaderboards                |
| [/users/{username}/activity/data](#activity_data)       | GET    | Geting user or user friend coding activity data |
| [/users/{username}/activity/summary](#activity_summary) | GET    | Get a summary of a users activity               |
| [/users/{username}/activity/current](#activity_cur)     | GET    | Get a users current coding session              |
| [/users/@me/delete](#delete_myself)                     | DELETE | Deleting user account                           |

#### <a name="me"></a>  [1. GET /users/@me](#users)

Gets data about authorized user

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
</details>

**Sample request**
```curl
curl --location --request GET 'https://api.testaustime.fi/users/@me' \
--header 'Authorization: Bearer `<token>`'
```

**Sample response**
```JSON
{
    "id": 0,
    "friend_code": "friend_code",
    "username": "username",
    "registration_time": "YYYY-MM-DDTHH:MM:SS.ssssssZ"
}
```
<details>
  <summary>Response definitions:</summary>

| Response Item | Type | Description |
| --- | --- | --- |
| id | int| User id |
| friend_code | string | By this code another users can add user to the friend list |
| username | string | Username |
| registration_time | string (ISO 8601 format) | Time of registration to microseconds |
</details>

#### <a name="my_leaderboards"></a>  [2. GET /users/@me/leaderboards](#users)

Gets list of user leaderboards

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
</details>

**Sample request**
```curl
curl --location --request GET 'https://api.testaustime.fi/users/@me/leaderboards' \
--header 'Authorization: Bearer <token>'
```

**Sample response**
```JSON
[
    {
        "name": "Leaderboard name",
        "member_count": 2
    }
]
```

<details>
  <summary>Response definitions:</summary>

| Response Item | Type | Description |
| --- | --- | --- |
| name | string | Name of leaderboard in which the user is a member |
| member_count | int | Number of users in the leaderboard |
</details>

Required headers:
```
Authorization: Bearer <token>
```

#### <a name="activity_data"></a>  [3. GET /users/{username}/activity/data](#users)

Geting user or user friend coding activity data

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
</details>

<details>
  <summary>Path params:</summary>

| Path param |  Description |
| --- | --- |
| Username | Own or friend username. Also own username can be replaced on `@me`|
</details>

**Sample request**
```curl
curl --location --request GET 'https://api.testaustime.fi/users/@me/activity/data' \
--header 'Authorization: Bearer <token>'
```

**Sample response**
```JSON
[
    {
        "id": 0,
        "start_time": "YYYY-MM-DDTHH:MM:SS.ssssssZ",
        "duration": 0,
        "project_name": "project_name",
        "language": "language",
        "editor_name": "editor_name",
        "hostname": "hostname"
    }
]
```

<details>
  <summary>Response definitions:</summary>

| Response Item | Type | Description |
| --- | --- | --- |
| id | int | ID of user code session |
| start_time | string (ISO 8601 format) | Start time (time of sending first heartbeat) of user code session to microsecnods |
| duration | int | Duration of user code session in seconds |
| project_name | string| Name of the project in which user have a code session |
| language | string| Code language of the code session |
| editor_name | string| Name of IDA (Visual Studio Code, IntelliJ, Neovim, etc.) in which user is coding |
| hostname | string| User hostname |
</details>

#### <a name="activity_summary"></a>  [4. GET /users/{username}/activity/summary](#users)

Get a summary of a users activity

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
</details>

<details>
  <summary>Path params:</summary>

| Path param | Description                                                           |
| ---        | ---                                                                   |
| Username   | Own or a friends username. Own username can be substituted with `@me` |
</details>

**Sample request**
```curl
curl --location --request GET 'https://api.testaustime.fi/users/@me/activity/summary' \
--header 'Authorization: Bearer <token>'
```

**Sample response**
```JSON
{
    "all_time": {
        "languages": {
            "c": 1000,
            "rust": 2000
        },
        "total": 3000
    },
    "last_month": {
        "languages": {
            "c": 100,
            "rust": 200,
        }
        "total": 300
    },
    "last_week": {
        "languages": {
            "c": 10,
            "rust": 20
        },
        "total": 30
    }
}
```

<details>
  <summary>Response definitions:</summary>

| Response Item | Type                     | Description                                                                       |
| ---           | ---                      | ---                                                                               |
| all_time      | Object                   | All time coding activity summary for the user                                     |
| languages     | Object                   | Contains fields named after languages that have the coding time as thier value    |
| total         | int                      | The total coding time of the given period                                         |
| last_month    | Object                   | Similar to `all_time`                                                             |
| last_week     | Object                   | Similar to `all_time` and `last_month`                                            |
</details>

#### <a name="activity_cur"></a>  [5. GET /users/{username}/activity/current](#users)

Gets details of the ongoing coding session if there is one.

<details>
  <summary>Header params:</summary>

| Name          | Value            |
| ---           | ---              |
| Authorization | Bearer `<token>` |
</details>

<details>
  <summary>Path params:</summary>

| Path param |  Description |
| --- | --- |
| Username | Own or a friends username. Own username can also be replaced with `@me`|
</details>

**Sample request**
```curl
curl --request GET 'https://api.testaustime.fi/users/@me/activity/current' \
--header 'Authorization: Bearer <token>'
```

**Sample response**
```JSON
{
    "started": "YYYY-MM-DDTHH:MM:SS.ssssssZ",
    "duration": "10",
    "heartbeat": {
        "language": "c",
        "hostname": "hostname1",
        "editor_name": "Neovim",
        "project_name": "cool_project22"
    }
}
```

<details>
  <summary>Response definitions:</summary>

| Response Item | Type                     | Description                                                                       |
| ---           | ---                      | ---                                                                               |
| started       | string (ISO 8601 format) | Start time (time of sending first heartbeat) of user code session to microsecnods |
| duration      | int                      | Duration of user code session in seconds                                          |
| heartbeat     | Object                   | The HeartBeat object described [here](#activity_up)                               |
</details>

#### <a name="delete_myself"></a>  [6. DELETE /users/@me/delete](#users)

Deletes user account

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Content-Type | application/json |
</details>

<details>
  <summary>Body params:</summary>

| Param |  Type | Required | Description |
| --- | --- | --- | --- |
| username| string | Yes | Username |
| password | string | Yes | User password |
</details>

**Sample request**
```curl
curl --request DELETE 'https://api.testaustime.fi/users/@me/delete' \
--header 'Content-Type: application/json' \
--data-raw '{
    "username": "username",
    "password": "password"
}'
```

**Sample response**
```http
200 OK
```

## <a name="activity"></a>  Activity

Contains main operations with activity heartbeats on which this service is based on

### Endpoints

| Endpoint                             | Method | Description                                             |
| ---                                  | ---    | ---                                                     |
| [/activity/update](#activity_up)     | POST   | Creating code session and logs current activity in that |
| [/activity/flush](#activity_fl)      | POST   | Flushing any currently active coding session            |
| [/activity/rename](#activity_rename) | POST   | Rename all activities with matching `project_name`      |
| [/activity/delete](#activity_del)    | DELETE | Deleting selected code session                          |

#### <a name="activity_up"></a>  [1. POST /activity/update](#activity)

Main endpoint of the service. Creates code session and logs current activity in that.

>*The desired interval at which to send heartbeats is immediately when editing a file, and after that at max every 30 seconds, and only when the user does something actively in the editor*

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
| Content-Type | application/json |
</details>

<details>
  <summary>Body params:</summary>

| Param | Type | Description |
| --- | --- | --- |
| language | string | Code language of the code session  |
| hostname | string | User hostname |
| editor_name | string | Name of IDA (Visual Studio Code, IntelliJ, Neovim, etc.) in which user is coding |
| project_name | string| Name of the project in which user have a code session |
</details>

**Sample first request**

If the user doesn't have any active code session with this set of body params, then first request `POST /activity/update` creates new code session. Any other code session automatically stops/flushes after starting new one, so the user can't have >1 active code sessions in one time

```curl
curl --request POST 'https://api.testaustime.fi/activity/update' \
--header 'Authorization: Bearer <token>' \
--header 'Content-Type: application/json' \
--data-raw '{
    "language": "Python",
    "hostname": "Hostname1",
    "editor_name": "IntelliJ",
    "project_name": "example_project"
}'
```

**Sample first response**
```HTTP
200 OK
```

**Sample next request**

```curl
curl --request POST 'https://api.testaustime.fi/activity/update' \
--header 'Authorization: Bearer <token>' \
--header 'Content-Type: application/json' \
--data-raw '{
    "language": "Python",
    "hostname": "Hostname1",
    "editor_name": "IntelliJ",
    "project_name": "example_project"
}'
```

**Sample next response**
```HTTP
200 OK
Body: PT7.420699439S //duration of the user code session in seconds to nanoseconds
```

#### <a name="activity_fl"></a>  [2. POST /activity/flush](#activity)

Flushes/stops any currently active coding session

>*Active coding session can be flushed/stoped automatically without any activity updates for a long time. Also can be flushed automatically in case of starting new code session*

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
</details>

**Sample request**
```curl
curl --request POST 'https://api.testaustime.fi/activity/flush' \
--header 'Authorization: Bearer <token>'
```

**Sample response**
```HTTP
200 OK
```

#### <a name="activity_rename"></a>  [3. POST /activity/rename](#activity)

Rename all activities that have a matching `project_name`

<details>
  <summary>Header params:</summary>

| Name          | Value            |
| ---           | ---              |
| Content-Type  | application/json |
| Authorization | Bearer `<token>` |
</details>

<details>
  <summary>Body params:</summary>
| Param | Type   | Required | Description |
| ---   | ---    | ---      | ---         |
| from  | string | Yes      | old name    |
| to    | string | Yes      | new name    |
</details>


**Sample request**
```curl
curl --request POST 'https://api.testaustime.fi/activity/rename' \
--header 'Content-Type: application/json' \
--header 'Authorization: Bearer <token>' \
--data-raw '{
    "from": "old_name",
    "to": "new_name"
}'
```

**Sample response**
```JSON
{
    "affected_activities": 20
}
```

<details>
  <summary>Response definitions:</summary>
| Response Item       | Type | Description                  |
| ---                 | ---  | ---                          |
| affected_activities | int  | Number of activities renamed |
</details>

#### <a name="activity_del"></a>  [4. POST /activity/delete](#activity)

Deletes selected code session

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
</details>

<details>
  <summary>Body params:</summary>

| Param    | Type   | Description                                                                       |
| ---      | ---    | ---                                                                               |
| raw text | string | Activity id from response [`GET /users/{username}/activity/data`](#activity_data) |
</details>

**Sample request**
```curl
curl --request DELETE 'https://api.testaustime.fi/activity/delete' \
--header 'Authorization: Bearer <token>' \
--data-raw 'activity_id'
```

**Sample response**
```HTTP
200 OK
```

## <a name="friends"></a>  Friends

Containts CRUD-operations with user friends

### Endpoints

| Endpoint|  Method | Description |
| --- | --- | --- |
| [/friends/add](#add_friend) | POST | Adding the holder of the friend_token as a friend of authorized user |
| [/friends/list](#list_friends) | GET | Geting a list of added user friends |
| [/friends/regenerate](#regenerate_fc) | POST | Regenerateing the authorized user's friend code |
| [/friends/remove](#remove_friend) | DELETE | Removing another user from user friend list |

#### <a name="add_friend"></a>  [1. POST /friends/add](#friends)

Adds the holder of the friend token as a friend of the authenticating user

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
</details>

<details>
  <summary>Body params:</summary>

| Param | Type | Description |
| --- | --- | --- |
| raw text | string | Should contain friend code without any prefixes |
</details>

**Sample request**

```curl
curl --request POST 'https://api.testaustime.fi/friends/add' \
--header 'Authorization: Bearer <token>' \
--data-raw 'friend_code'
```

**Sample response**
```HTTP
200 OK
```
<details>
  <summary>Error examples:</summary>

| Error | Error code | Body |
| --- | --- | --- |
| Friendcode is already used for adding a friend | 403 Forbidden | { "error": "Already friends"} |
| Friendcode from body request is not found | 404 Not Found | { "error": "User not found"} |
| Friendcode matches with friendcode of authorized user himself the n 403 Forbidden | 403 Forbidden | { "error": "You cannot add yourself"} |
</details>

#### <a name="list_friends"></a>  [2. GET friends/list](#friends)

Gets a list of added user friends

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
</details>

**Sample request**

```curl
curl --request GET ''https://api.testaustime.fi/friends/list' \
--header 'Authorization: Bearer <token>'
```

**Sample response**
```JSON
[
    {
        "username": "username",
        "coding_time": {
            "all_time": 0,
            "past_month": 0,
            "past_week": 0
        }
    }
]
```

<details>
  <summary>Response definitions:</summary>

| Response Item | Type | Description |
| --- | --- | --- |
| username | string | Friend's username |
| coding_time | Object | Coding friend's time by total, past month and past week |
| all_time | int | Total duration of user code sessions in seconds |
| past_month | int| Total duration of user code sessions in seconds for past month |
| past_week | int| Total duration of user code sessions in seconds for past week |
</details>

#### <a name="regenerate_fc"></a>  [3. POST /friends/regenerate](#friends)

Regenerates the authorized user's friend code

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
</details>

**Sample request**
```curl
curl --request POST 'https://api.testaustime.fi/friends/regenerate' \
--header 'Authorization: Bearer <token>'
```

**Sample response**
```JSON
{
    "friend_code": "friend_code"
}
```

<details>
  <summary>Response definitions:</summary>

| Response Item | Type | Description |
| --- | --- | --- |
| friend_code | string| New friend code. Using for the all next create friends paire operations |
</details>

#### <a name="remove_friend"></a>  [4. DELETE /friends/remove](#friends)

Removes another user from your friend list

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
</details>

<details>
  <summary>Body:</summary>

| Param | Type | Description |
| --- | --- | --- |
| raw text | string | Should contain username without any prefixes |
</details>

**Sample request**
```curl
curl --request DELETE 'https://api.testaustime.fi/friends/remove' \
--header 'Authorization: Bearer <token>' \
--data-raw 'username'
```

**Sample response**
```HTTP
200 OK
```

## <a name="leaderboards"></a>  Leaderboards

Containts CRUD-operations with leaderboards consisting of other Testaustime users

### Endpoints

| Endpoint|  Method | Description |
| --- | --- | --- |
| [/leaderboards/create](#create_lb) | POST | Adding new leaderboard |
| [/leaderboard/join](#join_lb) | POST | Joining leaderboard by it's invite code |
| [/leaderboards/{name}](#read_lb) | GET | Getting info about leaderboard if authorized user is a member |
| [/leaderboard/{name}](#delete_lb) | DELETE | Deleting leaderboard if authorized user has admin rights |
| [/leaderboards/{name}/leave](#leave_lb) | POST | Leaving the leaderboard |
| [/leaderboards/{name}/regenerate](#regenerate_lb) | POST | Regenerating invite code of the leaderboard if authorized user has admin rights |
| [/leaderboards/{name}/promote](#promote_lb) | POST | Promoting member of a leaderboard to admin if authorized user has admin rights |
| [/leaderboards/{name}/demote](#demote_lb) | POST | Demoting promoted admin to regular member of the leaderboard if authorized user has admin rights |
| [/leaderboards/{name}/kick](#kick_lb) | POST | Kicking user from leaderboard if authorized user has root admin rights |

#### <a name="create_lb"></a>  [1. POST /leaderboards/create](#leaderboards)

Adds new leaderboard

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
| Content-Type | application/json |
</details>

<details>
  <summary>Body params:</summary>

| Param | Type | Description |
| --- | --- | --- |
| name| string | Name of creating leaderboard |
</details>

**Sample request**
```curl
curl --request POST 'https://api.testaustime.fi/leaderboards/create' \
--header 'Authorization: Bearer <token>' \
--header 'Content-Type: application/json' \
--data-raw '{
    "name": "<name>"
}'
```

**Sample response**
```JSON
{
    "invite_code": "invite_code"
}
```
<details>
  <summary>Response definitions:</summary>

| Response Item | Type | Description |
| --- | --- | --- |
| invite_code | string| Invite code for joining leaderboard |
</details>

<details>
  <summary>Error examples:</summary>

| Error | Error code | Body |
| --- | --- | --- |
| "Name" of the leaderboard is already used| 403 Forbidden | { "error": "Leaderboard exists"} |
</details>

#### <a name="join_lb"></a>  [2. POST /leaderboard/join](#leaderboards)

Joins leaderboard by it's invite code

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
| Content-Type | application/json |
</details>

<details>
  <summary>Body params:</summary>

| Param | Type | Description |
| --- | --- | --- |
| invite | string | Invite code for joining leaderboard |
</details>

**Sample request**

```curl
curl --request POST 'https://api.testaustime.fi/leaderboards/join' \
--header 'Authorization: Bearer <token>' \
--header 'Content-Type: application/json' \
--data-raw '{
    "invite": "invite_code"
}'
```

**Sample response**
```JSON
{
    "member_count": 0,
    "name": "name"
}

```

<details>
  <summary>Response definitions:</summary>

| Response Item | Type | Description |
| --- | --- | --- |
| member_count | int| Number of leaderboard members |
| name | string| Leaderboard name |
</details>

<details>
  <summary>Error examples:</summary>

| Error | Error code | Body |
| --- | --- | --- |
| Authorized user is already part of the leaderboard | 403 Forbidden | { "error": "You're already a member"} |
| Leaderboard not found by invite code | 404 Not Found | { "error": "Leaderboard not found"} |
</details>

#### <a name="read_lb"></a>  [3. GET /leaderboards/{name}](#leaderboards)

Gets info about leaderboard if authorized user is a member

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
</details>

<details>
  <summary>Path params:</summary>

| Path param | Description |
| --- | --- |
| {name} | Leaderboard name |
</details>

**Sample request**

```curl
curl --request GET 'https://api.testaustime.fi/leaderboards/{name}' \
--header 'Authorization: Bearer <token>'
```

**Sample response**
```JSON
{
  "name": "name",
  "invite": "invite_code",
  "creation_time": "YYYY-MM-DDTHH:MM:SS.ssssssZ",
  "members": [
    {
      "username": "username",
      "admin": true,
      "time_coded": 0
    }
  ]
}
```
<details>
  <summary>Response definitions:</summary>

| Response Item | Type | Description |
| --- | --- | --- |
| name | int| Leaderboard name |
| invite | int| Invite code for joining leaderboard |
| creation_time| string (ISO 8601 format) | Time of leaderboard creation to microsends |
| members | array object| Information about leaderboard members |
| username| string| Member username|
| admin | boolean| Rights of leaderboard member: admin or regular |
| time_coded | int| Total duration of user code sessions in second |
</details>

<details>
  <summary>Error examples:</summary>

| Error | Error code | Body |
| --- | --- | --- |
| Authorized user is not part of this leaderboard | 401 Unauthorized | { "error": "You are not authorized"} |
| Leaderboard not found by name | 404 Not Found | { "error": "Leaderboard not found"} |
</details>

#### <a name="delete_lb"></a>  [4. DELETE /leaderboard/{name}](#leaderboards)

Deletes leaderboard if authorized user has admin rights

>*Note: Leaderboard can be deleted either by root administrator or by promoted one*

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
</details>

<details>
  <summary>Path params:</summary>

| Path param | Description |
| --- | --- |
| {name} | Leaderboard name |
</details>

**Sample request**

```curl
curl --request DELETE 'https://api.testaustime.fi/leaderboards/{name}' \
--header 'Authorization: Bearer <token>'
```

**Sample response**
```HTTP
200 OK
```

<details>
  <summary>Error examples:</summary>

| Error | Error code | Body |
| --- | --- | --- |
| Authorized user is not part of this leaderboard | 401 Unauthorized | { "error": "You are not authorized"} |
| Leaderboard not found by name | 404 Not Found | { "error": "Leaderboard not found"} |
</details>

#### <a name="leave_lb"></a>  [5. POST /leaderboards/{name}/leave](#leaderboards)

Leaves the leaderboard

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
</details>

<details>
  <summary>Path params:</summary>

| Path param | Description |
| --- | --- |
| {name} | Leaderboard name |
</details>

**Sample request**

```curl
curl --request POST 'https://api.testaustime.fi/leaderboards/{name}/leave' \
--header 'Authorization: Bearer <token>'
```

**Sample response**
```HTTP
200 OK
```

<details>
  <summary>Error examples:</summary>

| Error | Error code | Body |
| --- | --- | --- |
| Authorized user is the last admin in leaderboard| 403 Forbidden | { "error": "There are no more admins left, you cannot leave"} |
| User is not the part of the leaderboard | 403 Frobidden | { "error": "You're not a member"} |
| Leaderboard not found by name | 404 Not Found | { "error": "Leaderboard not found"} |
</details>

#### <a name="promote_lb"></a>  [6. POST /leaderboards/{name}/regenerate](#leaderboards)

Regenerates invite code of the leaderboard if authorized user has admin rights

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Authorization | Bearer `<token>` |
</details>

<details>
  <summary>Path params:</summary>

| Path param | Description |
| --- | --- |
| {name} | Leaderboard name |
</details>

**Sample request**

```curl
curl --request POST 'https://api.testaustime.fi/leaderboards/{name}/regenerate' \
--header 'Authorization: Bearer <token>'
```

**Sample response**
```JSON
{
    "invite_code": "<invite_code>"
}
```
<details>
  <summary>Error examples:</summary>

| Error | Error code | Body |
| --- | --- | --- |
| Authorized user is not part of found leaderboard or user is not an admin | 401 Unauthorized | { "error": "You are not authorized"} |
| Leaderboard not found by name | 404 Not Found | { "error": "Leaderboard not found"} |
</details>

#### <a name="regenerate_lb"></a>  [7. POST /leaderboards/{name}/promote](#leaderboards)

Promotes member of a leaderboard to admin if authorized user has admin rights. Be careful of promoting users, root admin (creator of the leaderboard) can be demoted/kicked as a promoted one

>*This request is idempotent, it means that you can:
>1. *Promote user that is already admin and have in response 200 OK*
>2. *Promote yourself to admin being already admin and have in response 200 OK*

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Content-Type | application/json |
| Authorization | Bearer `<token>` |
</details>

<details>
  <summary>Path params:</summary>

| Path param | Description |
| --- | --- |
| {name} | Leaderboard name |
</details>

<details>
  <summary>Body params:</summary>

| Param | Type | Description |
| --- | --- | --- |
| user | string | Username of a leaderboard member you want to promote |
</details>

**Sample request**

```curl
curl --request POST 'https://api.testaustime.fi/leaderboards/{name}/promote' \
--header 'Content-Type: application/json' \
--header 'Authorization: Bearer <token>' \
--data-raw '{
    "user": "<user>"
}'
```

**Sample response**
```HTTP
200 OK

```
<details>
  <summary>Error examples:</summary>

| Error | Error code | Body |
| --- | --- | --- |
| Authorized user is not part of found leaderboard or user is not an admin | 401 Unauthorized | { "error": "You are not authorized"} |
| Promoting user is not the leaderboard member | 403 Forbidden | { "error": "You're not a member"} |
</details>

#### <a name="demote_lb"></a>  [8. POST /leaderboards/{name}/demote](#leaderboards)

Demotes admin to regular member in the leaderboard if authorized user has admin rights. Be careful of promoting users, root admin (creator of the leaderboard) can be demoted as a promoted one

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Content-Type | application/json |
| Authorization | Bearer `<token>` |
</details>

<details>
  <summary>Path params:</summary>

| Path param | Description |
| --- | --- |
| {name} | Leaderboard name |
</details>

<details>
  <summary>Body params:</summary>

| Param | Type | Description |
| --- | --- | --- |
| user | string | Username of a leaderboard admin you want to demote|
</details>

**Sample request**

```curl
curl --request POST 'https://api.testaustime.fi/leaderboards/{name}/demote' \
--header 'Content-Type: application/json' \
--header 'Authorization: Bearer <token>' \
--data-raw '{
    "user": "<user>"
}'
```

**Sample response**
```HTTP
200 OK

```

<details>
  <summary>Error examples:</summary>

| Error | Error code | Body |
| --- | --- | --- |
| Authorized user is not part of found leaderboard or user is not an admin | 401 Unauthorized | { "error": "You are not authorized"} |
| Demoting user is not the leaderboard member | 403 Forbidden | { "error": "You're not a member"} |
</details>

#### <a name="kick_lb"></a>  [9. POST /leaderboards/{name}/kick](#leaderboards)

Kicks user from leaderboard if authorized user has admin rights

<details>
  <summary>Header params:</summary>

| Name |  Value |
| --- | --- |
| Content-Type | application/json |
| Authorization | Bearer `<token>` |
</details>

<details>
  <summary>Path params:</summary>

| Path param | Description |
| --- | --- |
| {name} | Leaderboard name |
</details>

<details>
  <summary>Body params:</summary>

| Param | Type | Description |
| --- | --- | --- |
| user | string | Username of a leaderboard member you want to kick|
</details>

**Sample request**

```curl
curl --request POST 'https://api.testaustime.fi/leaderboards/{name}/kick' \
--header 'Content-Type: application/json' \
--header 'Authorization: Bearer <token>' \
--data-raw '{
    "user": "<user>"
}'
```

**Sample response**
```HTTP
200 OK

```
<details>
  <summary>Error examples:</summary>

| Error | Error code | Body |
| --- | --- | --- |
| Authorized user is not part of found leaderboard or user is not an admin | 401 Unauthorized | { "error": "You are not authorized"} |
| Kicking user is not the leaderboard member | 403 Forbidden | { "error": "You're not a member"} |
</details>
