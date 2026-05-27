import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import L from "leaflet";
import "leaflet.markercluster";
import countries from "i18n-iso-countries";
import en from "i18n-iso-countries/langs/en.json";
import type {
    As,
    AsFilters,
    AsFiltersHasOrg,
    AsForFrontend,
    AsrankDegree,
    Bound
} from "./protocol/types";
import { getAllAsFiltered, getAsDetails } from "./api/ws";

countries.registerLocale(en);

const POLAND_LAT = 52.11431;
const POLAND_LON = 19.423672;
const MARKER_ICON_URL = "https://unpkg.com/leaflet@1.9.3/dist/images/marker-icon.png";

const DEFAULT_FILTERS: AsFilters = {
    country: "PL",
    exclude_country: false,
    bounds: null,
    addresses: [0, 21000000],
    rank: [0, 115000],
    has_org: "Both",
    category: []
};

function formatDegree(degree: AsrankDegree): string {
    return `provider:${degree.provider},peer:${degree.peer},customer:${degree.customer},total:${degree.total},transit:${degree.transit},sibling:${degree.sibling}`;
}

function formatFilters(filters: AsFilters): string {
    const bounds = filters.bounds;
    const boundStr = bounds
        ? `b${bounds.south_west.lat.toFixed(4)}:${bounds.south_west.lon.toFixed(4)}-${bounds.north_east.lat.toFixed(4)}:${bounds.north_east.lon.toFixed(4)}`
        : "";
    const addresses = filters.addresses ?? [0, 0];
    const rank = filters.rank ?? [0, 0];
    const hasOrg = filters.has_org === "Both" ? "both" : filters.has_org === "Yes" ? "yes" : "no";

    return `c${filters.country ?? ""}-exc${filters.exclude_country}-${boundStr}-a${addresses[0]}-${addresses[1]}-r${rank[0]}-${rank[1]}-org${hasOrg}-ncat${filters.category.length}`;
}

function csvEscape(value: string): string {
    if (value.includes("\"") || value.includes(",") || value.includes("\n")) {
        return `"${value.replaceAll("\"", "\"\"")}"`;
    }
    return value;
}

function scaleAsMarker(rank: number): [number, number] {
    const rankRangeMax = 115000;
    const avgPixels: [number, number] = [15, 24];
    const minPixels: [number, number] = [5, 8];
    const scale = Math.min(Math.max(rank / rankRangeMax, 0), 1);

    const width = minPixels[0] + avgPixels[0] - Math.round(avgPixels[0] * scale);
    const height = minPixels[1] + avgPixels[1] - Math.round(avgPixels[1] * scale);
    return [width, height];
}

function getCountryName(code: string): string {
    return countries.getName(code, "en") ?? "";
}

function buildBasePopup(asn: AsForFrontend): string {
    const org = asn.organization ?? "none";
    const country = getCountryName(asn.country_code);
    return `<b>asn</b>:${asn.asn} <b>rank</b>:${asn.rank} <b>prefixes</b>:${asn.prefixes} <b>addresses</b>:${asn.addresses}<br>
<b>links</b>:<a href="https://bgp.he.net/AS${asn.asn}" target="_blank">bgp.he</a>, shodan<br>
<b>name</b>:${asn.name}<br>
<b>org</b>:${org}<br>
<b>country</b>:${country}`;
}

function buildTooltip(asn: AsForFrontend): string {
    return `AS${asn.asn}:${asn.name}:${asn.organization ?? ""}`;
}

function buildDetailsHtml(details: As, baseHtml: string): string {
    let html = baseHtml;
    if (details.asrank_data?.degree) {
        html += `<br><b>degree</b>: ${formatDegree(details.asrank_data.degree)}`;
    }

    if (details.ipnetdb_data?.ipv4_prefixes?.length) {
        const prefixes = details.ipnetdb_data.ipv4_prefixes;
        const prefixQuery = prefixes.map((p) => `${p.range},`).join("");
        html = html.replace(
            "shodan",
            `<a href="https://www.shodan.io/search?query=net:${prefixQuery}" target="_blank">shodan</a>`
        );

        const prefixLinks = prefixes
            .map((p) => {
                const cidr = p.range;
                return `${cidr}:<b><a href="https://www.shodan.io/search?query=net%3A${cidr}" target="_blank">s</a></b>|` +
                    `<b><a href="https://www.zoomeye.org/searchResult?q=cidr%3A${cidr}" target="_blank">z</a></b>|` +
                    `<b><a href="https://search.censys.io/search?resource=hosts&sort=RELEVANCE&per_page=25&virtual_hosts=EXCLUDE&q=ip%3A${cidr}" target="_blank">c</a></b> `;
            })
            .join("");

        html += `<br><b>prefixes</b>: ${prefixLinks}`;
    }

    if (details.stanford_asdb?.length) {
        const categories = new Set(details.stanford_asdb.map((c) => c.layer1));
        const categoryHtml = Array.from(categories)
            .map((c) => `<b>></b>${c}<b>.</b><br>`)
            .join("");
        html += `<br><b>categories</b>: ${categoryHtml}`;
    }

    return html;
}

export default function MapView() {
    const mapContainerRef = useRef<HTMLDivElement | null>(null);
    const mapRef = useRef<L.Map | null>(null);
    const clusterRef = useRef<L.MarkerClusterGroup | null>(null);
    const markersByAsnRef = useRef<Map<number, L.Marker>>(new Map());
    const drawnAsRef = useRef<Map<number, AsForFrontend>>(new Map());
    const detailedAsRef = useRef<Map<number, As>>(new Map());

    const [filters, setFilters] = useState<AsFilters>(DEFAULT_FILTERS);
    const [prevFilters, setPrevFilters] = useState<AsFilters>(DEFAULT_FILTERS);
    const [drawnCount, setDrawnCount] = useState(0);
    const [detailedCount, setDetailedCount] = useState(0);

    const updateCounts = useCallback(() => {
        setDrawnCount(drawnAsRef.current.size);
        setDetailedCount(detailedAsRef.current.size);
    }, []);

    useEffect(() => {
        if (!mapContainerRef.current || mapRef.current) {
            return;
        }

        const map = L.map(mapContainerRef.current, { zoomControl: true });
        map.setMaxZoom(18);
        map.setView([POLAND_LAT, POLAND_LON], 5);

        L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png").addTo(map);

        const cluster = L.markerClusterGroup({
            maxClusterRadius: (zoom) => (zoom < 9 ? 80 : 1),
            chunkedLoading: true
        });
        cluster.addTo(map);

        mapRef.current = map;
        clusterRef.current = cluster;

        setTimeout(() => map.invalidateSize(), 0);
    }, []);

    const updateMarkerPopup = useCallback((asn: number, details: As) => {
        const marker = markersByAsnRef.current.get(asn);
        if (!marker) {
            return;
        }

        const popup = marker.getPopup();
        if (!popup) {
            return;
        }

        const baseHtml = (popup.getContent() as string) ?? "";
        const updated = buildDetailsHtml(details, baseHtml);
        popup.setContent(updated);
    }, []);

    const handlePopupOpen = useCallback(async (asn: number) => {
        if (detailedAsRef.current.has(asn)) {
            updateMarkerPopup(asn, detailedAsRef.current.get(asn) as As);
            return;
        }

        try {
            const details = await getAsDetails(asn);
            detailedAsRef.current.set(asn, details);
            updateMarkerPopup(asn, details);
            updateCounts();
        } catch (error) {
            console.error(error);
        }
    }, [updateCounts, updateMarkerPopup]);

    const drawAses = useCallback((ases: AsForFrontend[]) => {
        const cluster = clusterRef.current;
        if (!cluster) {
            return;
        }

        ases.forEach((asn) => {
            if (drawnAsRef.current.has(asn.asn)) {
                return;
            }

            drawnAsRef.current.set(asn.asn, asn);

            const [width, height] = scaleAsMarker(asn.rank);
            const marker = L.marker([asn.coordinates.lat, asn.coordinates.lon], {
                icon: L.icon({
                    iconUrl: MARKER_ICON_URL,
                    iconSize: [width, height]
                })
            });

            marker.bindPopup(L.popup({ maxWidth: 600 }).setContent(buildBasePopup(asn)));
            marker.bindTooltip(buildTooltip(asn));
            marker.on("popupopen", () => handlePopupOpen(asn.asn));

            markersByAsnRef.current.set(asn.asn, marker);
            cluster.addLayer(marker);
        });

        updateCounts();
    }, [handlePopupOpen, updateCounts]);

    const loadBoundsOnly = useCallback(async () => {
        const map = mapRef.current;
        if (!map) {
            return;
        }

        const bounds = map.getBounds();
        const requestFilters: AsFilters = {
            country: null,
            exclude_country: false,
            bounds: {
                north_east: { lat: bounds.getNorthEast().lat, lon: bounds.getNorthEast().lng },
                south_west: { lat: bounds.getSouthWest().lat, lon: bounds.getSouthWest().lng }
            },
            addresses: null,
            rank: null,
            has_org: "Both",
            category: []
        };

        try {
            const ases = await getAllAsFiltered(requestFilters);
            drawAses(ases);
        } catch (error) {
            console.error(error);
        }
    }, [drawAses]);

    const loadFiltered = useCallback(async () => {
        const map = mapRef.current;
        if (!map) {
            return;
        }

        let requestFilters = { ...filters };
        if (requestFilters.bounds) {
            const bounds = map.getBounds();
            requestFilters = {
                ...requestFilters,
                bounds: {
                    north_east: { lat: bounds.getNorthEast().lat, lon: bounds.getNorthEast().lng },
                    south_west: { lat: bounds.getSouthWest().lat, lon: bounds.getSouthWest().lng }
                }
            };
        }

        setPrevFilters(requestFilters);

        try {
            const ases = await getAllAsFiltered(requestFilters);
            drawAses(ases);
        } catch (error) {
            console.error(error);
        }
    }, [drawAses, filters]);

    const clearMap = useCallback(() => {
        drawnAsRef.current.clear();
        detailedAsRef.current.clear();
        markersByAsnRef.current.clear();
        clusterRef.current?.clearLayers();
        updateCounts();
    }, [updateCounts]);

    const downloadCsv = useCallback((detailed: boolean) => {
        const timestamp = Math.floor(Date.now() / 1000);
        let filename = "";
        let csv = "";

        if (detailed) {
            const values = Array.from(detailedAsRef.current.values());
            filename = `asmap-detailed-${values.length}-${timestamp}.csv`;
            const header = "asn,rank,name,organization";
            const rows = values.map((as) => {
                const asrank = as.asrank_data;
                return [
                    String(as.asn),
                    String(asrank?.rank ?? ""),
                    csvEscape(asrank?.name ?? ""),
                    csvEscape(asrank?.organization ?? "")
                ].join(",");
            });
            csv = [header, ...rows].join("\n");
        } else {
            const values = Array.from(drawnAsRef.current.values());
            const filterTag = formatFilters(prevFilters);
            filename = `asmap-${values.length}-${filterTag}-${timestamp}.csv`;
            const header = "asn,rank,name,organization";
            const rows = values.map((as) => {
                return [
                    String(as.asn),
                    String(as.rank),
                    csvEscape(as.name),
                    csvEscape(as.organization ?? "")
                ].join(",");
            });
            csv = [header, ...rows].join("\n");
        }

        const blob = new Blob([csv], { type: "text/plain" });
        const url = URL.createObjectURL(blob);
        const link = document.createElement("a");
        link.href = url;
        link.download = filename;
        link.click();
        URL.revokeObjectURL(url);
    }, [prevFilters]);

    const toggleBounded = useCallback(() => {
        setFilters((current) => {
            if (current.bounds) {
                return { ...current, bounds: null };
            }
            const placeholder: Bound = {
                north_east: { lat: 0, lon: 0 },
                south_west: { lat: 0, lon: 0 }
            };
            return { ...current, bounds: placeholder };
        });
    }, []);

    const updateHasOrg = useCallback((value: string) => {
        const next = value === "yes" ? "Yes" : value === "no" ? "No" : "Both";
        setFilters((current) => ({ ...current, has_org: next as AsFiltersHasOrg }));
    }, []);

    const updateCategories = useCallback((selected: string[]) => {
        setFilters((current) => ({ ...current, category: selected }));
    }, []);

    const categoryOptions = useMemo(
        () => [
            "Any",
            "Computer and Information Technology",
            "Media, Publishing, and Broadcasting",
            "Finance and Insurance",
            "Education and Research",
            "Service",
            "Agriculture, Mining, and Refineries (Farming, Greenhouses, Mining, Forestry, and Animal Farming)",
            "Community Groups and Nonprofits",
            "Construction and Real Estate",
            "Museums, Libraries, and Entertainment",
            "Utilities (Excluding Internet Service)",
            "Health Care Services",
            "Travel and Accommodation",
            "Freight, Shipment, and Postal Services",
            "Government and Public Administration",
            "Retail Stores, Wholesale, and E-commerce Sites",
            "Manufacturing",
            "Other",
            "Unknown"
        ],
        []
    );

    return (
        <div className="min-h-screen bg-slate-50 text-slate-100 flex flex-col md:flex-row gap-4 p-4">
            <div className="flex-1 min-h-[60vh] rounded-xl border border-slate-800 shadow-lg overflow-hidden">
                <div ref={mapContainerRef} className="h-full" />
            </div>

            <div className="w-full md:w-96 space-y-4">
                <div className="p-4 rounded-xl border border-slate-800 bg-slate-900/60 shadow">
                    <div className="flex flex-col gap-2">
                        <button
                            onClick={loadBoundsOnly}
                            className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 active:bg-blue-800 text-white text-sm font-medium rounded-lg transition-colors duration-150"
                        >
                            {"Load visible range"}
                        </button>
                        <button
                            onClick={loadFiltered}
                            className="w-full px-4 py-2 bg-green-600 hover:bg-green-700 active:bg-green-800 text-white text-sm font-medium rounded-lg transition-colors duration-150"
                        >
                            {"Apply filters →"}
                        </button>
                        <button
                            onClick={() => downloadCsv(false)}
                            className="w-full px-4 py-2 bg-slate-700 hover:bg-slate-600 active:bg-slate-500 text-slate-200 text-sm font-medium rounded-lg transition-colors duration-150"
                        >
                            {"Download loaded"}
                        </button>
                        <button
                            onClick={() => downloadCsv(true)}
                            className="w-full px-4 py-2 bg-slate-700 hover:bg-slate-600 active:bg-slate-500 text-slate-200 text-sm font-medium rounded-lg transition-colors duration-150"
                        >
                            {"Download detailed"}
                        </button>
                        <button
                            onClick={clearMap}
                            className="w-full px-4 py-2 bg-red-600 hover:bg-red-700 active:bg-red-800 text-white text-sm font-medium rounded-lg transition-colors duration-150"
                        >
                            {"Clear map"}
                        </button>
                    </div>
                </div>

                <div className="p-4 rounded-xl text-sm border border-slate-800 bg-slate-900/60 shadow">
                    <div className="space-y-4">
                        <div className="p-3 rounded-lg bg-slate-800/50 border border-slate-700">
                            <div className="flex items-center gap-2 pt-1">
                                <input
                                    type="checkbox"
                                    id="isBounded"
                                    checked={Boolean(filters.bounds)}
                                    className="w-4 h-4 bg-slate-900 border-slate-600 rounded focus:ring-2 focus:ring-blue-500"
                                    onChange={toggleBounded}
                                />
                                <label htmlFor="isBounded" className="text-xs text-slate-400 cursor-pointer">
                                    {"Bound to visible area"}
                                </label>
                            </div>
                        </div>

                        <div className="p-3 rounded-lg bg-slate-800/50 border border-slate-700">
                            <div className="space-y-2">
                                <label className="block text-xs font-medium text-slate-400">{"Min Addresses"}</label>
                                <input
                                    type="number"
                                    id="minAddresses"
                                    value={filters.addresses?.[0] ?? 0}
                                    min={0}
                                    max={99999999}
                                    className="w-full px-3 py-1.5 bg-slate-900 border border-slate-600 rounded text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    onChange={(e) =>
                                        setFilters((current) => ({
                                            ...current,
                                            addresses: [Number(e.target.value), current.addresses?.[1] ?? 0]
                                        }))
                                    }
                                />

                                <label className="block text-xs font-medium text-slate-400 mt-3">{"Max Addresses"}</label>
                                <input
                                    type="number"
                                    id="maxAddresses"
                                    value={filters.addresses?.[1] ?? 0}
                                    min={0}
                                    max={99999999}
                                    className="w-full px-3 py-1.5 bg-slate-900 border border-slate-600 rounded text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    onChange={(e) =>
                                        setFilters((current) => ({
                                            ...current,
                                            addresses: [current.addresses?.[0] ?? 0, Number(e.target.value)]
                                        }))
                                    }
                                />
                            </div>
                        </div>

                        <div className="p-3 rounded-lg bg-slate-800/50 border border-slate-700">
                            <div className="space-y-2">
                                <label className="block text-xs font-medium text-slate-400">{"Country Code"}</label>
                                <input
                                    type="text"
                                    id="countryCode"
                                    value={filters.country ?? ""}
                                    maxLength={2}
                                    placeholder="PL"
                                    className="w-20 px-3 py-1.5 bg-slate-900 border border-slate-600 rounded text-sm text-slate-200 uppercase focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    onChange={(e) =>
                                        setFilters((current) => ({
                                            ...current,
                                            country: e.target.value ? e.target.value.toUpperCase() : null
                                        }))
                                    }
                                />

                                <div className="flex items-center gap-2 mt-2">
                                    <input
                                        type="checkbox"
                                        id="excludeCountry"
                                        checked={filters.exclude_country}
                                        className="w-4 h-4 bg-slate-900 border-slate-600 rounded focus:ring-2 focus:ring-blue-500"
                                        onChange={() =>
                                            setFilters((current) => ({
                                                ...current,
                                                exclude_country: !current.exclude_country
                                            }))
                                        }
                                    />
                                    <label htmlFor="excludeCountry" className="text-xs text-slate-400 cursor-pointer">
                                        {"Exclude country"}
                                    </label>
                                </div>
                            </div>
                        </div>

                        <div className="p-3 rounded-lg bg-slate-800/50 border border-slate-700">
                            <div className="space-y-2">
                                <label className="block text-xs font-medium text-slate-400">{"Min Rank"}</label>
                                <input
                                    type="number"
                                    id="minRank"
                                    value={filters.rank?.[0] ?? 0}
                                    min={0}
                                    max={999999}
                                    className="w-24 px-3 py-1.5 bg-slate-900 border border-slate-600 rounded text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    onChange={(e) =>
                                        setFilters((current) => ({
                                            ...current,
                                            rank: [Number(e.target.value), current.rank?.[1] ?? 0]
                                        }))
                                    }
                                />

                                <label className="block text-xs font-medium text-slate-400 mt-3">{"Max Rank"}</label>
                                <input
                                    type="number"
                                    id="maxRank"
                                    value={filters.rank?.[1] ?? 0}
                                    min={0}
                                    max={999999}
                                    className="w-24 px-3 py-1.5 bg-slate-900 border border-slate-600 rounded text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    onChange={(e) =>
                                        setFilters((current) => ({
                                            ...current,
                                            rank: [current.rank?.[0] ?? 0, Number(e.target.value)]
                                        }))
                                    }
                                />
                            </div>
                        </div>

                        <div className="p-3 rounded-lg bg-slate-800/50 border border-slate-700">
                            <div className="space-y-3">
                                <div>
                                    <label className="block text-xs font-medium text-slate-400 mb-2">{"Has Organization"}</label>
                                    <select
                                        id="hasOrg"
                                        name="hasOrgSel"
                                        className="w-full px-3 py-1.5 bg-slate-900 border border-slate-600 rounded text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                        value={filters.has_org === "Yes" ? "yes" : filters.has_org === "No" ? "no" : "both"}
                                        onChange={(e) => updateHasOrg(e.target.value)}
                                    >
                                        <option value="yes">{"Yes"}</option>
                                        <option value="no">{"No"}</option>
                                        <option value="both">{"Both"}</option>
                                    </select>
                                </div>
                            </div>
                        </div>

                        <div className="p-3 rounded-lg bg-slate-800/50 border border-slate-700">
                            <label className="block text-xs font-medium text-slate-400 mb-2">{"Category"}</label>
                            <select
                                id="category"
                                name="category"
                                multiple
                                className="w-full h-40 px-3 py-2 bg-slate-900 border border-slate-600 rounded text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                value={filters.category.length ? filters.category : ["Any"]}
                                onChange={(e) => {
                                    const selected = Array.from(e.target.selectedOptions).map((o) => o.value);
                                    updateCategories(selected.includes("Any") ? [] : selected);
                                }}
                            >
                                {categoryOptions.map((category) => (
                                    <option key={category} value={category}>
                                        {category}
                                    </option>
                                ))}
                            </select>
                        </div>
                    </div>
                </div>

                <div className="p-3 rounded-lg border border-slate-800 bg-slate-900/60 text-sm">
                    <div className="space-y-1">
                        <div className="flex justify-between">
                            <span className="font-semibold text-slate-400">{"Drawn:"}</span>
                            <span className="text-slate-200">{drawnCount}</span>
                        </div>
                        <div className="flex justify-between">
                            <span className="font-semibold text-slate-400">{"Detailed:"}</span>
                            <span className="text-slate-200">{detailedCount}</span>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
