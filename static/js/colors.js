const makeColorParam = (name, color) => {
    const input = document.createElement('input');
    input.type = 'color'
    input.value = color;
    input.addEventListener('input', () => {
        storedColors[name] = input.value;
        document.documentElement.style.setProperty(name, input.value);
        storeColors();
    });

    const element = document.createElement('li');
    element.textContent = name;
    element.appendChild(input);
    return element;
};

const populateColorsList = () => {
    const colorsList = document.getElementById('colors-list');
    colorsList.appendChild(makeColorParam('--background', storedColors['--background']));
};

window.addEventListener('load', populateColorsList);
