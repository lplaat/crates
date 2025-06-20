/*
 * Copyright (c) 2025 Bastiaan van der Plaat
 *
 * SPDX-License-Identifier: MIT
 */

export function send(type, data = {}) {
    console.log(`Send ${type}`);
    window.ipc.postMessage(JSON.stringify({ type, ...data }));
}

export function request(type, data = {}) {
    return new Promise((resolve) => {
        const messageListener = (event) => {
            const { type: receivedType, ...data } = JSON.parse(event.data);
            console.log(`Recv ${receivedType}`);
            if (receivedType === `${type}-response`) {
                window.ipc.removeEventListener('message', messageListener);
                resolve(data);
            }
        };
        window.ipc.addEventListener('message', messageListener);
        send(type, data);
    });
}
