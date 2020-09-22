const printStatus = function() {
    const serverStatus = document.getElementById('serverStatus');
    fetch('/session_count')
        .then(response => {
            if (response.status !== 200) {
                throw new Error('Did not receive session count');
            }
            return response.text();
        })
        .then(count => {
            serverStatus.textContent = `Active sessions: ${count}.`;
        })
        .catch(error => {
            serverStatus.textContent = error;
        });
};

const placeRooms = function() {
    const makeRoomListItem = (name) => {
        const description = document.createElement('p');
        description.textContent = name;
        description.style.display = 'inline-block';
        description.style.margin = '10px';

        const deleteButton = document.createElement('input');
        deleteButton.setAttribute('type', 'button');
        deleteButton.setAttribute('value', 'Delete room!');
        deleteButton.addEventListener('click', () => deleteRoom(name));

        const passwordInput = document.createElement('input');
        passwordInput.setAttribute('type', 'text');

        const passwordButton = document.createElement('input');
        passwordButton.setAttribute('type', 'button');
        passwordButton.setAttribute('value', 'Change password!');
        passwordButton.addEventListener('click', () => {
            const newPassword = passwordInput.value.trim();
            if (newPassword) {
                changePassword(name, newPassword);
            } else {
                alert('The new password cannot be empty.');
            }
        });

        const item = document.createElement('li');
        item.appendChild(description);
        item.appendChild(deleteButton);
        item.appendChild(passwordInput);
        item.appendChild(passwordButton);
        return item;
    };

    const roomsBox = document.getElementById('roomsBox');
    roomsBox.innerHTML = '<p>Active rooms:</p>';

    fetch('/active_rooms')
        .then(response => {
            if (response.status !== 200) {
                throw new Error('Could not retrieve active rooms.');
            }
            return response.json();
        })
        .then(rooms => {
            const roomsList = document.createElement('ol');
            for (const name of rooms) {
                roomsList.appendChild(makeRoomListItem(name));
                roomsList.appendChild(document.createElement('hr'));
            }
            roomsBox.appendChild(roomsList);
        })
        .catch(error => {
            roomsBox.textContent += error;
        });
};

const deleteRoom = function(name) {
    // Make sure we want to delete the room.
    if (!window.confirm(`Do you really want to delete room ${name}?`)) {
        return;
    }

    fetch('/delete_room', {
        method: 'DELETE',
        body: name
    })
    .then(response => {
        if (!response.ok) {
            throw new Error('Network error?');
        }
        return response.text();
    })
    .then(status => {
        alert(status);
        placeRooms();
    })
    .catch(error => alert(error));
};

const changePassword = function(name, password) {
    // Make sure we want to change the password.
    if (!window.confirm(
        `Do you really want to change the password of room ${name}? ` +
        `This might log some users out.`
    )) {
        return;
    }

    fetch('/change_room_password', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({
            name: name,
            password: password
        })
    })
    .then(response => {
        if (!response.ok) {
            throw new Error('Network error?');
        }
        return response.text();
    })
    .then(status => {
        alert(status);
        placeRooms();
    })
    .catch(error => alert(error));
}

window.addEventListener('load', printStatus);
window.addEventListener('load', placeRooms);

window.addEventListener('load', function() {
    const form = document.getElementById('createRoomForm');
    form.onsubmit = async (e) => {
        e.preventDefault();

        const data = {
            name: form.elements['name'].value,
            password: form.elements['password'].value
        };

        fetch('/create_room', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(data)
        })
        .then(response => {
            if (!response.ok) {
                throw new Error('There was a network error.');
            }
            return response.text();
        })
        .then(status => {
            document.getElementById('createRoomStatus').textContent = status;
            placeRooms();
        })
        .catch(error => {
            document.getElementById('createRoomStatus').textContent = error;
        });
    };
});
