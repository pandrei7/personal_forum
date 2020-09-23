window.addEventListener('load', function() {
    const form = document.getElementById('loginForm');

    form.onsubmit = (e) => {
        e.preventDefault();
        fetch('/admin_login', {
            method: 'POST',
            redirect: 'follow',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                username: form.elements['username'].value,
                password: form.elements['password'].value
            })
        })
        .then(response => {
            if (!response.ok) {
                console.log(response.statusText);
            } else {
                window.location.replace(response.url);
            }
        });
    };
});
