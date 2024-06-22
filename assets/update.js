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
    /** @type {{ id: string, name: string, memory_usage: string, cpu_usage: string, exited: boolean }[]} */
    const data = JSON.parse(event.data);

    const now = Date.now();
    console.log("time since last update", now - lastUpdate);
    lastUpdate = now;

    for (const service of data) {
        updateCell(service);
    }
}

/**
 * update the cell with the new data
 * @param {{ id: string, name: string, memory_usage: string, cpu_usage: string, exited:boolean }} data 
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
        const memory = service.cells[1];
        const cpu = service.cells[2];

        memory.textContent = data.memory_usage;
        cpu.textContent = data.cpu_usage;

        return;
    }

    // add a new service to the list
    const table = document.querySelector('table');

    if (!table) {
        return;
    }

    console.log(data);

    // create a new row at the end of the table
    const row = table.insertRow(-1);
    row.id = data.id;

    // add new cells to the row
    const name = row.insertCell(0);
    const memory = row.insertCell(1);
    const cpu = row.insertCell(2);

    // set the cell values
    name.textContent = data.name;
    memory.textContent = data.memory_usage;
    cpu.textContent = data.cpu_usage;
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
}

connectSocket();