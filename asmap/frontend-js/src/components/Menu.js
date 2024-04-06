// the menu controlling the Map

import { useState } from "react";

export default function Menu() {
    const [isBounded, setIsBounded] = useState(false);
    const [country, setCountry] = useState("PL");

    return (
        <>
            "is bounded"
            <input type="checkbox" id="isBounded_id" title="is bounded?" onClick={
                () => {
                    setIsBounded(true)
                    console.log("toggled isbounded")
                }}
                value={isBounded} />
            "country"
            <input type="text" id="country_id" title="country" />
        </>
    )
}