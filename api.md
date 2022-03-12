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

### GET /activity/data

Get your coding activity data

Url params:
- language
- editor
- project_name
- hostname
- min_duration

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
Content-type: application/json
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
