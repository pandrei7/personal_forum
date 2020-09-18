const makeBox = function(message) {
    const text = document.createElement('p');
    text.innerHTML = `${message.content}`;

    const meta = document.createElement('p');
    meta.innerHTML =
        `(comment <strong>#${message.id}</strong> ` +
        `made by <strong>${message.author}</strong> ` +
        `at ${message.timestamp})`;

    const extra = document.createElement('p');
    extra.innerHTML =
        message.reply_to == null ? '>>> Starts a new thread'
                                 : `>>> In reply to ${message.reply_to}`;

    const separator = document.createElement('hr');

    const box = document.createElement('div');
    box.appendChild(text);
    box.appendChild(meta);
    box.appendChild(extra);
    box.appendChild(separator);
    return box;
};

const loadAllMessages = function() {
    fetch(`/room/${roomName}/updates`)
    .then(response => {
        if (!response.ok) {
            throw new Error('Network error while fetching updates.');
        }
        return response.json();
    })
    .then(messages => {
        const messageBox = document.getElementById('message-box');
        messageBox.innerHTML = "";

        messages.sort((a, b) => b.id - a.id);
        for (const message of messages) {
            messageBox.appendChild(makeBox(message));
        }
    });
};

const showInfo = function(info) {
    const box = document.getElementById('infoBox');
    box.textContent = info;
};

window.addEventListener('load', () => {
    document.getElementById('messageForm').onsubmit = async (e) => {
        e.preventDefault();

        const form = document.getElementById('messageForm');
        const data = {
            content: form.elements['content'].value,
            // Message numbers start at 1.
            reply_to: Number(form.elements['reply_to'].value) || null,
        };

        fetch(`/room/${roomName}/post`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(data)
        })
        .then(response => {
            if (!response.ok) {
                throw new Error('Could not save your message.');
            }
            return response.text();
        })
        .then(info => {
            showInfo(info);
            loadAllMessages();
        })
        .catch(error => showInfo(error));
    };
});

window.addEventListener('load', loadAllMessages);
