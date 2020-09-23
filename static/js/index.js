window.addEventListener('load', function() {
    const form = document.getElementById('loginForm');

    form.onsubmit = (e) => {
        e.preventDefault();
        fetch('/enter_room', {
            method: 'POST',
            redirect: 'follow',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                name: form.elements['name'].value,
                password: form.elements['password'].value
            })
        })
        .then(response => {
            if (!response.ok) {
                console.log('Network error?');
            } else {
                window.location.replace(response.url);
            }
        });
    };
});
