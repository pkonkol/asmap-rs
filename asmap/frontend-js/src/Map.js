import React, { useRef } from "react";
import { MapContainer, TileLayer } from "react-leaflet";
import "leaflet/dist/leaflet.css";
import Menu from "./components/Menu";

const Map = () => {
    const mapRef = useRef(null);
    // geometric center of poland
    const latitude = 52.11431;
    const longitude = 19.423672;

    return (
        <>
            <MapContainer center={[latitude, longitude]} zoom={13} ref={mapRef} style={{ height: "90vh", width: "90vw" }}>
                <TileLayer
                    attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
                    url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
                />

            </MapContainer>
            <Menu />
        </>
    );
};

export default Map;