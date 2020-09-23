let storedMessages = [];
let storedOpen = new Set();

class Thread {
    #firstPost = null;
    #replies = [];

    constructor(firstPost) {
        this.#firstPost = firstPost;
    }

    addReply(reply) {
        this.#replies.push(reply);
    }

    id() {
        return this.#firstPost.id;
    }

    getAsElement() {
        // Replies should appear in chronological order.
        this.#replies.sort((a, b) => a.timestamp - b.timestamp);

        const threadId = this.id();

        const repliesBox = document.createElement('div');
        repliesBox.classList.add('replies-box');
        repliesBox.hidden = !storedOpen.has(threadId);
        for (const reply of this.#replies) {
            repliesBox.appendChild(makeMessageBox(reply));
        }
        repliesBox.appendChild(makeSendBox(threadId));

        const firstPost = document.createElement('a');
        const firstMessageBox = makeMessageBox(this.#firstPost);
        firstMessageBox.classList.add('threadMessage');
        firstPost.appendChild(firstMessageBox);
        firstPost.addEventListener('click', function() {
            if (repliesBox.hidden) {
                repliesBox.hidden = false;
                storedOpen.add(threadId);
            } else {
                repliesBox.hidden = true;
                storedOpen.delete(threadId);
            }
        });

        const threadBox = document.createElement('div');
        threadBox.classList.add('thread-box');
        threadBox.appendChild(firstPost);
        threadBox.appendChild(repliesBox);

        return threadBox;
    }
}

const makeMessageBox = function(message) {
    const id = document.createElement('p');
    id.setAttribute('class', 'messageId');
    id.textContent = `#${message.id}.`;

    const content = document.createElement('div');
    content.setAttribute('class', 'messageContent');
    content.innerHTML = message.content;

    const box = document.createElement('div');
    box.classList.add('message');
    box.appendChild(id);
    box.appendChild(content);
    return box;
};

const makeSendBox = function(threadId) {
    const infoBox = document.createElement('p');

    const form = document.createElement('form');
    form.innerHTML = `
        <textarea name="content" placeholder="Write your reply here. You can use CommonMark." required></textarea>
        <input type="submit" value="Send!">
    `;
    form.onsubmit = async (e) => {
        e.preventDefault();

        fetch(`/room/${roomName}/post`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                content: form.elements['content'].value,
                reply_to: threadId
            })
        })
        .then(response => {
            if (!response.ok) {
                throw new Error('Could not save your message.');
            }
            return response.text();
        })
        .then(info => {
            infoBox.textContent = info;
            console.log(info);
            updateMessages();
        })
        .catch(error => console.log(error));
    };

    const sendBox = document.createElement('div');
    sendBox.classList.add('send-box');
    sendBox.appendChild(infoBox);
    sendBox.appendChild(form);
    return sendBox;
}

const updateMessages = function() {
    return fetch(`/room/${roomName}/updates`)
           .then(response => {
               if (!response.ok) {
                   throw new Error('Network error while fetching updates.');
               }
               return response.json();
           })
           .then(updates => {
               if (updates.clean_stored) {
                   localStorage.removeItem(`room${roomName}`);
                   localStorage.removeItem(`open${roomName}`);
                   storedMessages = [];
                   storedOpen = new Set();
               }
               storedMessages.push(...updates.messages);
               placeMessages(storedMessages);
           });
};

const placeMessages = function(messages) {
    messages.sort((a, b) => a.timestamp - b.timestamp);

    const threads = new Map();
    for (const message of messages) {
        if (message.reply_to == null) {
            threads.set(message.id, new Thread(message));
        }
    }
    for (const message of messages) {
        if (message.reply_to != null) {
            threads.get(message.reply_to).addReply(message);
        }
    }

    const messageBox = document.getElementById('message-box');
    messageBox.innerHTML = "";
    for (const [id, thread] of threads.entries()) {
        messageBox.appendChild(thread.getAsElement());
    }
};

const showInfo = function(info) {
    const box = document.getElementById('infoBox');
    box.textContent = info;
};

window.addEventListener('load', () => {
    storedMessages = JSON.parse(localStorage.getItem(`room${roomName}`)) ?? [];
    storedOpen =
        new Set(JSON.parse(localStorage.getItem(`open${roomName}`)) ?? []);
});

window.addEventListener('load', async () => {
    const infoBox = document.getElementById('newThreadInfoBox');
    const form = document.getElementById('newThreadForm');

    form.onsubmit = async (e) => {
        e.preventDefault();

        fetch(`/room/${roomName}/post`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                content: form.elements['content'].value,
                reply_to: null
            })
        })
        .then(response => {
            if (!response.ok) {
                throw new Error('Could not save your message.');
            }
            return response.text();
        })
        .then(info => {
            infoBox.textContent = info;
            updateMessages();
        })
        .catch(error => infoBox.textContent = error);
    };
});

window.addEventListener('load', async () => {
    await updateMessages();
    document.documentElement.scrollTop = sessionStorage.getItem('y');
});

window.addEventListener('beforeunload', () => {
    localStorage.setItem(`room${roomName}`, JSON.stringify(storedMessages));
    localStorage.setItem(`open${roomName}`, JSON.stringify([...storedOpen]));

    sessionStorage.setItem('y', document.documentElement.scrollTop);
});
