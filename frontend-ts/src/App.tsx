import { BrowserRouter, Route, Routes } from "react-router-dom";
import MapView from "./MapView";
import DetailsPage from "./DetailsPage";

export default function App() {
    return (
        <BrowserRouter>
            <Routes>
                <Route path="/" element={<MapView />} />
                <Route path="/details/:id" element={<DetailsPage />} />
                <Route path="*" element={<div className="p-6 text-slate-200">{"404"}</div>} />
            </Routes>
        </BrowserRouter>
    );
}
