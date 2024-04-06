import React, { useEffect, useRef } from "react";
import { MapContainer, TileLayer, useMap } from "react-leaflet";
import { marker } from "leaflet";
import "leaflet/dist/leaflet.css";
import Menu from "./components/Menu";

// geometric center of poland
const latitude = 52.11431;
const longitude = 19.423672;

const Map = () => {
    // const mapRef = useRef(null);
    const map = useMap();

    // function test123() {
    //     var m = marker([52, 19]).addTo(map)
    // }

    return (
        <>
            <MapContainer center={[latitude, longitude]} zoom={13} style={{ height: "90vh", width: "90vw" }}>
                <TileLayer
                    attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
                    url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
                />

            </MapContainer>
            {/* <button title="huj" onClick={() => console.log(mapRef.current)}>Huj123</button> */}
            {/* <button onClick={test123}>test123</button> */}
            <Menu />
        </>
    );
};

export default Map;