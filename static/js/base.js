let storedColors = {};

const storeColors = () => {
    localStorage.setItem('colors', JSON.stringify(storedColors));
};

const loadStoredColors = () => {
    const style = getComputedStyle(document.body);

    storedColors = JSON.parse(localStorage.getItem('colors')) ?? {
        '--background': style.getPropertyValue('--background'),
    };
};

const applyStoredColors = () => {
    for (const [name, value] of Object.entries(storedColors)) {
        document.documentElement.style.setProperty(name, value);
    }
};

window.addEventListener('load', async () => {
    await loadStoredColors();
    applyStoredColors();
});
