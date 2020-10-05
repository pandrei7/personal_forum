/** @file Provides code which allows admins to control the server somewhat. */

/**
 * Returns certain status information about the server.
 * @return {Promise<object>} An object containing status information.
 */
const getServerStatus = async () => {
    let status = {};

    await Promise.all([
        fetch('/session_count')
            .then((response) => response.text())
            .then((count) => status.sessionCount = count),
    ]);

    return status;
};

/**
 * Displays the given status information on the page.
 * @param {object} status An object containing status information.
 */
const displayServerStatus = (status) => {
    const serverStatus = document.getElementById('server-status-box');
    serverStatus.innerHTML = `
        <p>Active sessions: ${status.sessionCount}</p>
    `;
};

/** Gets the list of active rooms from the server and displays it. */
const refreshRooms = async () => {
    const rooms = await fetch('/active_rooms').then((response) => response.json());
    displayActiveRooms(rooms);
};

/**
 * Displays the given list of rooms on the page.
 *
 * The list items will also include controls to manipulate each room.
 *
 * @param {Array<string>} rooms The list of room names.
 */
const displayActiveRooms = (rooms) => {
    /**
     * Creates a list item for the given room.
     * @param {string} name The name of the room.
     * @return {HTMLElement} The list item element.
     */
    const createListItem = (name) => {
        const roomName = document.createElement('p');
        roomName.classList.add('room-name');
        roomName.textContent = name;

        const input = document.createElement('input');
        input.type = 'text';
        input.classList.add('new-password');
        input.placeholder = 'Type the new password here.';

        const changeButton = document.createElement('button');
        changeButton.textContent = 'Change';
        changeButton.addEventListener('click', () => changePassword(name, input.value));

        const deleteButton = document.createElement('button');
        deleteButton.classList.add('delete-button');
        deleteButton.textContent = 'Delete room!';
        deleteButton.addEventListener('click', () => deleteRoom(name));

        const controls = document.createElement('div');
        controls.classList.add('room-controls');
        controls.appendChild(input);
        controls.appendChild(changeButton);
        controls.appendChild(deleteButton);

        const item = document.createElement('li');
        item.appendChild(roomName);
        item.appendChild(controls);
        return item;
    };

    const activeRoomsList = document.getElementById('active-rooms-list');
    activeRoomsList.innerHTML = '';
    for (const name of rooms) {
        activeRoomsList.appendChild(createListItem(name));
    }
};

/**
 * Sends a server request to delete a given room.
 * The user will be prompted before actually sending the request.
 * @param {string} name The name of the room to be deleted.
 */
const deleteRoom = (name) => {
    // Make sure we want to delete the room.
    if (!window.confirm(`Do you really want to delete room '${name}'?`)) {
        return;
    }

    fetch('/delete_room', {method: 'DELETE', body: name})
        .then((response) => response.text())
        .then((status) => alert(status))
        .then(() => refreshRooms());
};

/**
 * Sends a server request to change the password of a given room.
 * The user will be prompted before actually sending the request.
 * @param {string} name The name of the room whose password we want to change.
 * @param {string} password The new password.
 */
const changePassword = (name, password) => {
    if (!(password = password.trim())) {
        alert('The password cannot be empty.');
        return;
    }

    // Make sure we want to change the password.
    if (!window.confirm(
        `Do you really want to change the password of room '${name}'? ` +
        `This might log some users out.`
    )) {
        return;
    }

    fetch('/change_room_password', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/x-www-form-urlencoded',
        },
        body: urlencodePairs({
            name: name,
            password: password
        }),
    })
        .then((response) => response.text())
        .then((status) => alert(status))
        .then(() => refreshRooms());
}

/**
 * Converts the given (key, value) pairs into a URL-encoded string.
 * @param {object} pairsObject An object containing (key, value) pairs.
 * @return {string} The URL-encoded string.
 */
const urlencodePairs = (pairsObject) => {
    const encode = encodeURIComponent;
    return Object.entries(pairsObject)
        .map(([key, value]) => `${encode(key)}=${encode(value)}`)
        .join('&');
};

// Set up the form which allows admins to create new rooms.
window.addEventListener('load', () => {
    const info = document.getElementById('create-room-status');
    const form = document.getElementById('create-room-form');

    form.addEventListener('submit', (event) => {
        event.preventDefault();

        fetch('/create_room', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/x-www-form-urlencoded',
            },
            body: urlencodePairs({
                name: form.elements['name'].value,
                password: form.elements['password'].value,
            }),
        })
            .then((response) => response.text())
            .then((status) => info.textContent = status)
            .then(() => refreshRooms());
    });
});

// Fetch the server status information and display it after the page loads.
window.addEventListener('load', async () => {
    const status = await getServerStatus();
    displayServerStatus(status);
});

// Fetch and display all the active rooms when the page loads.
window.addEventListener('load', refreshRooms);
