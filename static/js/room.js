const LIGHT_THEME = {
    '--background': '#ffffff',
};
const DARK_THEME = {
    '--background': '#bebebe',
};

let storedMessages = [];
let storedOpen = new Set();
let storedRoomsSeen = new Set();

class Thread {
    firstPost = null;
    replies = [];

    constructor(firstPost) {
        this.firstPost = firstPost;
    }

    addReply(reply) {
        this.replies.push(reply);
    }

    id() {
        return this.firstPost.id;
    }

    timestamp() {
        return this.firstPost.timestamp;
    }

    getAsElement() {
        // Replies should appear in chronological order.
        this.replies.sort((a, b) => a.timestamp - b.timestamp);

        const threadId = this.id();

        const repliesBox = document.createElement('div');
        repliesBox.classList.add('replies-box');
        repliesBox.hidden = !storedOpen.has(threadId);
        for (const reply of this.replies) {
            repliesBox.appendChild(makeMessageBox(reply));
        }
        repliesBox.appendChild(makeSendBox(threadId));

        const firstPost = document.createElement('a');
        const firstMessageBox = makeMessageBox(this.firstPost);
        firstMessageBox.classList.add('thread-message');
        firstPost.appendChild(firstMessageBox);
        firstPost.addEventListener('click', function() {
            if (repliesBox.hidden) {
                repliesBox.hidden = false;
                storedOpen.add(threadId);
                storeOpen();
            } else {
                repliesBox.hidden = true;
                storedOpen.delete(threadId);
                storeOpen();
            }
        });

        const threadBox = document.createElement('div');
        threadBox.classList.add('thread-box');
        threadBox.appendChild(firstPost);
        threadBox.appendChild(repliesBox);

        return threadBox;
    }
}

const addMentions = (node) => {
    findAndReplaceDOMText(node, {
        find: /@\d+/g,
        replace: (tag) => {
            const replyId = tag.text.slice(1);
            const link = document.createElement('a');
            link.href = `#message${replyId}`;
            link.innerHTML = tag.text;
            return link;
        },
    });
};

const makeMessageBox = function(message) {
    const id = document.createElement('p');
    id.id = `message${message.id}`;
    id.setAttribute('class', 'message-id');
    id.textContent = `#${message.id}.`;

    const content = document.createElement('div');
    content.setAttribute('class', 'message-content');
    content.innerHTML = message.content;
    addMentions(content);

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
               storeMessages();
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

    const searchText = document.getElementById('search-text').value;

    class OrderedElement {
        element = {};
        timestamp = 0;
        matches = 0;

        constructor(element, timestamp) {
            this.element = element;
            this.timestamp = timestamp;
        }
    }

    const elements = [];
    for (const [id, thread] of threads.entries()) {
        const element = thread.getAsElement();
        const orderedElement = new OrderedElement(element, thread.timestamp());

        if (searchText) {
            const marker = new Mark(element);
            marker.mark(searchText, {
                done: function(matchCount) {
                    orderedElement.matches = matchCount;
                    elements.push(orderedElement);
                }
            });
        } else {
            elements.push(orderedElement);
        }
    }
    elements.sort((a, b) => {
        if (a.matches === b.matches) {
            return b.timestamp - a.timestamp;
        }
        return b.matches - a.matches;
    });

    const messageBox = document.getElementById('message-box');
    messageBox.innerHTML = "";
    for (const orderedElement of elements) {
        messageBox.appendChild(orderedElement.element);
    }
};

const showInfo = function(info) {
    const box = document.getElementById('info-box');
    box.textContent = info;
};

const toggleColorsDropdown = () => {
    document.getElementById('colors-dropdown-content').classList.toggle('dropdown-show');
};

const redirectToRoom = async (name) => {
    await storeMessages(); // Save everything before redirecting.
    location.assign(`/room/${name}`);
};

const populateRoomsSeen = () => {
    const datalist = document.getElementById('rooms-seen');
    datalist.innerHTML = '';

    for (const name of storedRoomsSeen) {
        const option = document.createElement('option');
        option.setAttribute('value', name);
        datalist.appendChild(option);
    }
};

const loadPersistent = () => {
    storedMessages = JSON.parse(localStorage.getItem(`room${roomName}`)) ?? [];
    storedOpen =
        new Set(JSON.parse(localStorage.getItem(`open${roomName}`)) ?? []);
    storedRoomsSeen =
        new Set(JSON.parse(localStorage.getItem('roomsSeen')) ?? []);
}

const storeMessages = () => {
    localStorage.setItem(`room${roomName}`, JSON.stringify(storedMessages));
};

const storeOpen = () => {
    localStorage.setItem(`open${roomName}`, JSON.stringify([...storedOpen]));
};

const storeRoomsSeen = () => {
    localStorage.setItem('roomsSeen', JSON.stringify([...storedRoomsSeen]));
};

const storeScroll = () => {
    sessionStorage.setItem('y', document.documentElement.scrollTop);
};

const changeTheme = async (colors) => {
    storedColors = colors;
    await storeColors();
    applyStoredColors();
};

window.addEventListener('load', async () => {
    await loadPersistent();
    storedRoomsSeen.add(roomName);
    await storeRoomsSeen();
    populateRoomsSeen();
});

window.addEventListener('load', async () => {
    const info = document.getElementById('new-thread-info');
    const form = document.getElementById('new-thread-form');

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
        .then(response => {
            info.textContent = response;
            updateMessages();
        })
        .catch(error => info.textContent = error);
    };
});

window.addEventListener('load', () => {
    const goToRoom = document.getElementById('go-to-room');
    goToRoom.addEventListener('keyup', async (event) => {
        event.preventDefault();

        const wantedRoomName = goToRoom.value.trim();
        if (event.keyCode === 13 && wantedRoomName) {
            redirectToRoom(wantedRoomName);
        }
    });

    const textBox = document.getElementById('search-text');
    textBox.addEventListener('input', function() {
        placeMessages(storedMessages);
    });

    const clearSearchButton = document.getElementById('clear-search-button');
    clearSearchButton.addEventListener('click', function() {
        textBox.value = '';
        textBox.dispatchEvent(new Event('input'));
    });
});

window.addEventListener('load', async () => {
    await updateMessages();
    document.documentElement.scrollTop = sessionStorage.getItem('y');
});
