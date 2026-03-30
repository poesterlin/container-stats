// @ts-check

// connect to the socket server
const isHttps = window.location.protocol.startsWith('https');
const wsProtocol = isHttps ? 'wss' : 'ws';
const wsUrl = `${wsProtocol}://${window.location.host}/ws`;

/** @type {WebSocket} */
let ws;

let lastUpdate = Date.now();

/**
 * update the service list when a message is received
 * @param {MessageEvent} event 
 */
function listener(event) {
    /** @type {{ id: string, name: string, memory_usage: string, cpu_usage: string, disk_read: string, disk_write: string, exited: boolean }[]} */
    const data = JSON.parse(event.data);

    const now = Date.now();
    console.log("time since last update", now - lastUpdate);
    lastUpdate = now;

    for (const service of data) {
        updateCell(service);
    }

    const result = document.querySelector('#result');
    if (result) {
        setTimeout(() => {
            result.remove();
        }, 3000);
    }
}

/**
 * update the cell with the new data
 * @param {{ id: string, name: string, memory_usage: string, cpu_usage: string, disk_read: string, disk_write: string, exited:boolean }} data 
 */
function updateCell(data) {
    /** @type {HTMLTableRowElement | null} */
    const service = document.querySelector(`#${data.id}`);

    if (service) {
        if (data.exited) {
            // remove the service from the list
            service.remove();
            return;
        }


        // update the memory and cpu usage
        const memory = service.cells[2];
        const cpu = service.cells[3];
        const diskRead = service.cells[4];
        const diskWrite = service.cells[5];

        memory.textContent = data.memory_usage;
        cpu.textContent = data.cpu_usage;
        diskRead.textContent = data.disk_read;
        diskWrite.textContent = data.disk_write;

        return;
    }

    // add a new service to the list
    const table = document.querySelector('table');

    if (!table) {
        return;
    }

    // create a new row at the end of the table
    const row = table.insertRow(-1);
    row.id = data.id;

    // add new cells to the row
    const name = row.insertCell(0);
    const image = row.insertCell(1);
    const memory = row.insertCell(2);
    const cpu = row.insertCell(3);
    const diskRead = row.insertCell(4);
    const diskWrite = row.insertCell(5);

    // set the cell values
    name.textContent = data.name;
    memory.textContent = data.memory_usage;
    cpu.textContent = data.cpu_usage;
    diskRead.textContent = data.disk_read;
    diskWrite.textContent = data.disk_write;

    image.innerHTML = `<a href="?/restart=${data.id}"> <img src="/assets/reload.svg" alt="Restart ${data.name}" />`;
}

/**
 * reconnect to the server when the connection is closed
 */
function connectSocket() {
    ws = new WebSocket(wsUrl);

    ws.addEventListener('message', listener);
    ws.addEventListener('close', () => {
        setTimeout(connectSocket, 1000);
    });

    if (location.search.includes('restart')) {
        // remove the query string from the url
        history.pushState({}, document.title, location.pathname);
    }
}

connectSocket();
