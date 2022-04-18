# Testaustime-rs api documentation

## General info

- Ratelimit: 10 req/m

The desired interval at which to send heartbeats is immidiately when editing a file, and after that at max every 30 seconds, and only when the user does something actively in the editor.

## Endpoints

### POST /activity/update

Logs current activity.

This is the main endpoint this service is based on, this is where you current coding session data is sent.

Accepts:
```
{
    "language": string,
    "hostname": string,
    "editor_name": string,
    "project_name": string
}
```

Required headers:
```
Authorization: Bearer <token>
Content-type: application/json
```

### POST /activity/flush

Flushes any currently active coding session

Required headers:
```
Authorization: Bearer <token>
```

### GET /users/{username}/activity/data

Get your coding activity data

Url params:
- {username}
- language
- editor
- project_name
- hostname
- min_duration

The users with `{username}` has to be a friend or self of the auth_token provided

A special case of `{username}` is `@me` where the response will include the data of the authenticating user

Returns:
```
[
    {
        "language": string,
        "hostname": string,
        "editor_name": string,
        "project_name": string,
        "start_time": number,
        "duration": number
    },
    ...
]
```

Required headers:
```
Authorization: Bearer <token>
```

### GET /users/@me

Gets the data of the authenticating user

Returns:
```
{
    "id": int,
    "user_name": string,
    "friend_code": string,
    "registration_time": string,
}
```

Required headers:
```
Authorization: Bearer <token>
```

### POST /users/register

Registers a new user and returns the users auth token

Accepts:
```
{
    "username": string,
    "password": string
}
```

Required headers:
```
Content-type: application/json
```

Returns:
```
<AUTHTOKEN>
```

### POST /users/login

Logins to a users account returning the auth token

Accepts:
```
{
    "username": string,
    "password": string
}
```

Required headers:
```
Content-type: application/json
```

Returns:
```
<AUTHTOKEN>
```

### POST /users/changepassword

Changes the users password

Accepts:
```
{
    "old": string,
    "new": string
}
```

Required headers:
```
Authorization: Bearer <token>
Content-type: application/json
```

### POST /users/regenerate

Regenerate users auth token

Required headers:
```
Authorization: Bearer <token>
```

Returns:
```
<NEWAUTHTOKEN>
```

### POST /friends/add

Add the holder of the friend token as a friend of the authenticating user

Accepts:
```
ttfc_FRIENDCODE
```

*Note: The friend code is valid with or without the "ttfc_" prefix*


Required headers:
```
Authorization: Bearer <token>
```

### GET /friends/list

Gets a list of the authenticating users friends

Returns:
```
[
    string,
    ...
]
```

The string specifies the friends username

Required headers:
```
Authorization: Bearer <token>
```

### POST /friends/regenerate

Regenerates the authenticating users friend code


Required headers:
```
Authorization: Bearer <token>
```

Returns:
```
NEWFRIENDCODE
```
