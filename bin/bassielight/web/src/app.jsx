/*
 * Copyright (c) 2025 Leonard van der Plaat
 *
 * SPDX-License-Identifier: MIT
 */

import { useState, useEffect } from 'preact/hooks';

const COLORS = [0x000000, 0xff0000, 0x00ff00, 0x0000ff, 0xffff00, 0xff00ff, 0x00ffff, 0xffffff];
const SPEEDS = [null, 22, 50, 100, 200, 250, 500, 750, 1000];
const MODES = [
    {
        type: 'black',
        icon: (
            <path d="M12,2C9.76,2 7.78,3.05 6.5,4.68L7.93,6.11C8.84,4.84 10.32,4 12,4A5,5 0 0,1 17,9C17,10.68 16.16,12.16 14.89,13.06L16.31,14.5C17.94,13.21 19,11.24 19,9A7,7 0 0,0 12,2M3.28,4L2,5.27L5.04,8.3C5,8.53 5,8.76 5,9C5,11.38 6.19,13.47 8,14.74V17A1,1 0 0,0 9,18H14.73L18.73,22L20,20.72L3.28,4M7.23,10.5L12.73,16H10V13.58C8.68,13 7.66,11.88 7.23,10.5M9,20V21A1,1 0 0,0 10,22H14A1,1 0 0,0 15,21V20H9Z" />
        ),
    },
    {
        type: 'manual',
        icon: (
            <path d="M12,4A4,4 0 0,1 16,8A4,4 0 0,1 12,12A4,4 0 0,1 8,8A4,4 0 0,1 12,4M12,14C16.42,14 20,15.79 20,18V20H4V18C4,15.79 7.58,14 12,14Z" />
        ),
    },
    {
        type: 'auto',
        icon: (
            <path d="M21,3V15.5A3.5,3.5 0 0,1 17.5,19A3.5,3.5 0 0,1 14,15.5A3.5,3.5 0 0,1 17.5,12C18.04,12 18.55,12.12 19,12.34V6.47L9,8.6V17.5A3.5,3.5 0 0,1 5.5,21A3.5,3.5 0 0,1 2,17.5A3.5,3.5 0 0,1 5.5,14C6.04,14 6.55,14.12 7,14.34V6L21,3Z" />
        ),
    },
];

function send(type, data) {
    window.ipc.postMessage(JSON.stringify({ type, ...data }));
}

function capitalizeFirstLetter(string) {
    return string.charAt(0).toUpperCase() + string.slice(1);
}

export function App() {
    const [selectedColor, setSelectedColor] = useState(COLORS[0]);
    const [selectedToggleColor, setSelectedToggleColor] = useState(COLORS[0]);
    const [selectedToggleSpeed, setSelectedToggleSpeed] = useState(SPEEDS[0]);
    const [selectedStrobeSpeed, setSelectedStrobeSpeed] = useState(SPEEDS[0]);
    const [selectedMode, setSelectedMode] = useState(MODES[0].type);

    useEffect(() => {
        send('setColor', { color: selectedColor });
    }, [selectedColor]);
    useEffect(() => {
        send('setToggleColor', { color: selectedToggleColor });
    }, [selectedToggleColor]);
    useEffect(() => {
        send('setToggleSpeed', { speed: selectedToggleSpeed });
    }, [selectedToggleSpeed]);
    useEffect(() => {
        send('setStrobeSpeed', { speed: selectedStrobeSpeed });
    }, [selectedStrobeSpeed]);
    useEffect(() => {
        send('setMode', { mode: selectedMode });
    }, [selectedMode]);

    return (
        <>
            <h2>Color</h2>
            <div className="button-grid">
                {COLORS.map((color) => (
                    <button
                        key={color}
                        className={`color-button${color === selectedColor ? ' selected' : ''}`}
                        style={{ backgroundColor: `#${color.toString(16).padStart(6, '0')}` }}
                        onClick={() => setSelectedColor(color)}
                    />
                ))}
            </div>

            <h2>Toggle Colors</h2>
            <div className="button-grid">
                {COLORS.map((color) => (
                    <button
                        key={color}
                        className={`color-button${color === selectedToggleColor ? ' selected' : ''}`}
                        style={{ backgroundColor: `#${color.toString(16).padStart(6, '0')}` }}
                        onClick={() => setSelectedToggleColor(color)}
                    />
                ))}
            </div>

            <h2>Toggle Speeds</h2>
            <div className="button-grid">
                {SPEEDS.map((speed) => (
                    <button
                        key={speed}
                        className={`text-button${speed === selectedToggleSpeed ? ' selected' : ''}`}
                        onClick={() => setSelectedToggleSpeed(speed)}
                    >
                        {speed == null ? 'Off' : `${speed}ms`}
                    </button>
                ))}
            </div>

            <h2>Strobe Speeds</h2>
            <div className="button-grid">
                {SPEEDS.map((speed) => (
                    <button
                        key={speed}
                        className={`text-button${speed === selectedStrobeSpeed ? ' selected' : ''}`}
                        onClick={() => setSelectedStrobeSpeed(speed)}
                    >
                        {speed == null ? 'Off' : `${speed}ms`}
                    </button>
                ))}
            </div>

            <div className="bottom-controls-container">
                {MODES.map((mode) => (
                    <button
                        key={mode}
                        className={`control-button ${mode.type === selectedMode ? ' selected' : ''}`}
                        onClick={() => setSelectedMode(mode.type)}
                    >
                        <svg className="icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
                            {mode.icon}
                        </svg>
                        {capitalizeFirstLetter(mode.type)}
                    </button>
                ))}
            </div>
        </>
    );
}
