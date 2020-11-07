# API guide

This is a small guide for the API exposed by the server.

It should help developers work on the front-end of the web app.

## Table of contents

### Explanations

- [Sessions and authentication](#sessions-and-authentication)
- [Getting message updates](#getting-message-updates)

### HTTP calls

- [Authentication](#authentication)
  - [`POST /admin_login`](#post-admin_login)
  - [`POST /enter_room`](#post-enter_room)

- [Web pages](#web-pages)
  - [`GET /admin_login`](#get-admin_login)
  - [`GET /admin_pane`](#get-admin_pane)
  - [`GET /colors`](#get-colors)
  - [`GET /`](#get-)
  - [`GET /room/<name>`](#get-roomname)

- [Room functionality](#room-functionality)
  - [`GET /room/<name>/updates`](#get-roomnameupdates)
  - [`POST /room/<name>/post`](#post-roomnamepost)

- [Admin functionality](#admin-functionality)
  - [`GET /session_count`](#get-session_count)
  - [`GET /welcome_message`](#get-welcome_message)
  - [`POST /change_welcome_message`](#post-change_welcome_message)
  - [`GET /active_rooms`](#get-active_rooms)
  - [`POST /create_room`](#post-create_room)
  - [`DELETE /delete_room`](#delete-delete_room)
  - [`POST /change_room_password`](#post-change_room_password)

- [Other](#other)
  - [`GET /static/<path...>`](#get-staticpath)

## Sessions and authentication

The app relies on user sessions to provide its functionality. They are handled
almost entirely server-side, meaning that all data associated with a session
is kept on the server. The user receives an encrypted cookie called `session_id`
which only contains an identifier used by the server to refer to a session.
These session ids are sent on almost all requests which don't feature such a
cookie. Normal users don't need to log into accounts.

Users can obtain admin privileges by logging in using admin credentials.
Similarly, rooms are password-protected, and users need to log into them.
Permissions are set on the server by toggling some flags, the session cookie
does not change in any way.

Sessions expire after a period of inactivity. If a user keeps interacting with
the site, their session shouldn't expire. You can find more details in the
[source code](../src/sessions.rs).

## Getting message updates

The method of getting message updates to users was designed to reduce the
traffic between the server and its clients. To achieve this, the server keeps
track of the time when each user last requested updates for each given room.
Then, when a new update-request arrives, only new messages are sent.

This means that front-end implementations have to store all messages received
from previous requests at all times. In the browser, the
[Web Storage API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Storage_API)
should probably work fine.

Because messages are sent only once to clients, missing such a response might
break the functionality of the app. The only way of re-requesting the missing
messages is by clearing the session cookies to start a new session, which will
log users out of rooms. Clients should try to detect such situations and
instruct users about them.

## Authentication

These calls deal with logging users into rooms, or obtaining admin privileges.

### `POST /admin_login`

Obtain admin privileges.

The body should contain the admin credentials as URL-encoded strings,
in **plaintext**.

Content-Type must be `application/x-www-form-urlencoded`.

Fields:

- `username` the admin's username
- `password` the admin's password

### `POST /enter_room`

Log into a room.

The body should contain the room's credentials as URL-encoded strings,
in **plaintext**.

Content-Type must be `application/x-www-form-urlencoded`.

Fields:

- `name` the room's name
- `password` the room's password

## Web pages

These calls retrieves the site's HTML pages.

### `GET /admin_login`

Returns a page through which users can log in as admins.

### `GET /admin_pane`

Returns a page through which admins can control the server (the admin pane).

**Requires admin privileges.** If the session does not have admin privileges,
you are redirected to the [admin login page](#get-admin_login).

### `GET /colors`

Returns a page which allows users to customize their color scheme.

### `GET /`

Returns the index page of the site. This page should allow users to log into
rooms.

### `GET /room/<name>`

Returns the page corresponding to the room with the given name.

This page should allow the user to fully interact with the room, providing
operations such as: reading messages, replying, creating new threads etc.

**Requires valid credentials for the room.** If the user did not log into the
room, they are redirected to the [login page](#get-).

The `name` variable should be a valid room name. From the
[source code](../src/constraints.rs):
> Valid room names are not allowed to be empty. They also should not be too long.
>
> The only characters permitted are alphanumeric ASCII
> characters and a few "special" ones, such as: '\_' and '-'.

## Room functionality

These calls allow you to interact with rooms.

### `GET /room/<name>/updates`

Get all the messages posted to a room since the user's last request for updates.

**Requires valid credentials for the room.** If the user is not allowed to
access the room, a **401 Unauthorized** response is sent. If the server
experiences any issues, a **500 Internal Server Error** response is sent.

Since the server keeps track of update times and only sends new messages,
the responses should somehow be saved on the front-end to offer users a normal
way to interact with the site.

If everything works well, the server sends a JSON object with the following
structure:

```json
// Example reponse.
{
    "clean_stored": true,
    "messages": [
        {
            "content": "<p>Knock, knock!</p>",
            "id": 1,
            "reply_to": null,
            "timestamp": 1601413066627,
        },
        {
            "content": "<p>Who's there?</p>",
            "id": 2,
            "reply_to": 1,
            "timestamp": 1601661305463,
        },
    ],
}
```

Fields:

- `clean_stored` tells the client if it should remove the messages previously
    stored for this room. This is usualy `true` only the first time a client
    requests messages for a given room. This field is needed because rooms
    are identified by their names. If a room is deleted, and a new room with
    the same name is created, clients should remove all stored messages which
    belonged to the old room.
- `messages` the list containing the actual messages. Messages have the
    the following fields:
  - `content` an HTML string containing the actual message
  - `id` the numeric identifier of the message
  - `reply_to` the id of the message to whom this one replies. If the message
    starts a new thread, this field is `null`. This should be the id of a
    thread-starting message, **you cannot reply to another reply**.
  - `timestamp` a numeric timestamp of the moment when the server received
    this message. Messages received earlier have smaller timestamps.

### `POST /room/<name>/post`

Post a user message to the given room.

**Requires valid credentials for the room.** If the user is not allowed to
access the room, a **401 Unauthorized** response is sent. If the server
experiences any issues, a **500 Internal Server Error** response is sent.

The body must contain a JSON object string describing the messages to be posted.

If everything works correctly, a **200 OK** response is sent, containing a
human-readable status string. This string informs users about what happened
with their message (if it was saved etc.). Note that the server might reject
a message if it does not meet certain criteria (for example, if it's too long).

Content-Type must be `application/json`.

Fields:

- `content` a [CommonMark](https://commonmark.org) string representing the
    actual message. The string can contain HTML code too, the server will
    sanitize it.
- `reply_to` the id of the message you want to reply to. If you want to start
    a new thread, set this field to `null`. Keep in mind that **you can only
    reply to messages which start threads**.

Example:

```json
// Assume that this message will receive an id of 13.
{
    "content": "Let's start a new **thread**!",
    "reply_to": null,
}

// We can reply to it.
{
    "content": "<p>Sounds like a good idea.</p>",
    "reply_to": 13,
}
```

## Admin functionality

These calls allow admins to control the server and check its status.

**All these calls require admin privileges.**

### `GET /session_count`

Get the number of active sessions.

The number is represented as plaintext in the body of the response.

If the server experiences any issues, a **500 Internal Server Error** response
is sent.

### `GET /welcome_message`

Get the HTML welcome message displayed on the front page.

The message is returned as plaintext in the body of the response.

### `POST /change_welcome_message`

Change the welcome message displayed on the front page.

The body of the request should contain the HTML string of the new message,
in plaintext.

The server returns a human-readable string about the status of the operation.

Content-Type should be `text/plain; charset=utf-8`.

### `GET /active_rooms`

Get a list of all the rooms which exist currently.

The response contains a JSON array of strings, each string being the name of a room.

If the server experiences any issues, a **500 Internal Server Error** response
is sent.

### `POST /create_room`

Create a new room.

The body should contain the credentials of the new room as URL-encoded strings,
in **plaintext**.

The server returns a human-readable string about the status of the operation.
The room might not be created if, for example, the password is empty, or the
name contains invalid characters. You can find more details about room-name
constraints in the [source code](../src/constraints.rs). Also, attempts to
create a room which already exists will get rejected.

Content-Type must be `application/x-www-form-urlencoded`.

Fields:

- `name` the name of the new room
- `password` the password of the new room

### `DELETE /delete_room`

Delete an existing room.

The body should contain a string representing the name of an existing room.

The server returns a human-readable string about the status of the operation.

### `POST /change_room_password`

Change the password of a room.

The body must contain the new credentials of the room as URL-encoded strings,
in **plaintext**.

The server returns a human-readable string about the status of the operation.

Content-Type must be `application/x-www-form-urlencoded`.

Fields:

- `room` the valid name of the room
- `password` the value of the new password

## Other

These are calls which didn't fit into other categories.

### `GET /static/<path...>`

Retrieve a static resource.

The server exposes some static resources, such as HTML pages, CSS stylesheets,
or images. Mention a resource's path afther the `/static/` prefix to access it.

Since these resources are 'static', they do not usually change and can probably
be cached. The server response will include a Cache-Control header if that is
the case.

If the server cannot find the resource you requested, a **404 Not Found**
response is sent.
