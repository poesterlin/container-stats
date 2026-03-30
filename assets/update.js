// @ts-check

// connect to the socket server
const isHttps = window.location.protocol.startsWith('https');
const wsProtocol = isHttps ? 'wss' : 'ws';
const wsUrl = `${wsProtocol}://${window.location.host}/ws`;

/** @type {WebSocket} */
let ws;

let lastUpdate = Date.now();

/**
 * @param {'connected' | 'connecting' | 'disconnected'} state
 */
function setSocketStatus(state) {
    const el = document.querySelector('#ws-status');
    if (!el) {
        return;
    }

    const labels = {
        connected: 'Live',
        connecting: 'Connecting',
        disconnected: 'Disconnected'
    };

    el.textContent = labels[state];
    el.dataset.state = state;
}

function setLastUpdated() {
    const el = document.querySelector('#last-updated');
    if (!el) {
        return;
    }

    const now = new Date();
    const time = now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    el.textContent = time;
}

/**
 * update the service list when a message is received
 * @param {MessageEvent} event 
 */
function listener(event) {
    /** @type {{ id: string, name: string, memory_usage: string, cpu_usage: string, disk_read: string, disk_write: string, exited: boolean }[]} */
    const data = JSON.parse(event.data);

    lastUpdate = Date.now();
    setLastUpdated();

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
        const memory = service.cells[3];
        const cpu = service.cells[4];
        const diskRead = service.cells[5];
        const diskWrite = service.cells[6];

        memory.textContent = data.memory_usage;
        cpu.textContent = data.cpu_usage;
        diskRead.textContent = data.disk_read;
        diskWrite.textContent = data.disk_write;

        return;
    }

    // add a new service to the list
    const body = document.querySelector('tbody');

    if (!body) {
        return;
    }

    // create a new row at the end of the table
    const row = body.insertRow(-1);
    row.id = data.id;

    // add new cells to the row
    const name = row.insertCell(0);
    const restart = row.insertCell(1);
    const stop = row.insertCell(2);
    const memory = row.insertCell(3);
    const cpu = row.insertCell(4);
    const diskRead = row.insertCell(5);
    const diskWrite = row.insertCell(6);

    // set classes for consistency
    name.className = 'name-cell';
    restart.className = 'action-cell';
    stop.className = 'action-cell';

    // set the cell values
    name.textContent = data.name;
    memory.textContent = data.memory_usage;
    cpu.textContent = data.cpu_usage;
    diskRead.textContent = data.disk_read;
    diskWrite.textContent = data.disk_write;

    restart.innerHTML = `<a href="?restart=${data.id}"><img src="/assets/reload.svg" alt="Restart ${data.name}" /></a>`;
    stop.innerHTML = `<a href="?stop=${data.id}">Stop</a>`;
}

/**
 * reconnect to the server when the connection is closed
 */
function connectSocket() {
    setSocketStatus('connecting');
    ws = new WebSocket(wsUrl);

    ws.addEventListener('message', listener);
    ws.addEventListener('open', () => {
        setSocketStatus('connected');
        setLastUpdated();
    });
    ws.addEventListener('close', () => {
        setSocketStatus('disconnected');
        setTimeout(connectSocket, 1000);
    });

    if (location.search.includes('restart') || location.search.includes('stop')) {
        // remove the query string from the url
        history.pushState({}, document.title, location.pathname);
    }
}

connectSocket();
