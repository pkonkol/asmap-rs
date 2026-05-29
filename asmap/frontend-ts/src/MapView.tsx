import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import L from "leaflet";
import "leaflet.markercluster";
import type {
    As,
    AsFilters,
    AsFiltersHasOrg,
    AsForFrontend,
    Bound,
    UserData,
    WhoIsAsn
} from "./protocol/types";
import {
    fetchAsWhoisData,
    getAllAsFiltered,
    getAsDetails,
    getListNames,
    getUserData,
    updateUserData
} from "./api/ws";

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
    category: [],
    lists: []
};

function formatFilters(filters: AsFilters): string {
    const bounds = filters.bounds;
    const boundStr = bounds
        ? `b${bounds.south_west.lat.toFixed(4)}:${bounds.south_west.lon.toFixed(4)}-${bounds.north_east.lat.toFixed(4)}:${bounds.north_east.lon.toFixed(4)}`
        : "";
    const addresses = filters.addresses ?? [0, 0];
    const rank = filters.rank ?? [0, 0];
    const hasOrg = filters.has_org === "Both" ? "both" : filters.has_org === "Yes" ? "yes" : "no";

    return `c${filters.country ?? ""}-exc${filters.exclude_country}-${boundStr}-a${addresses[0]}-${addresses[1]}-r${rank[0]}-${rank[1]}-org${hasOrg}-ncat${filters.category.length}-nl${filters.lists.length}`;
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

function formatNumber(n: number): string {
    if (n >= 1_000_000) {
        return `${(n / 1_000_000).toFixed(1)}M`;
    }
    if (n >= 1_000) {
        return `${(n / 1_000).toFixed(1)}K`;
    }
    return n.toString();
}

function buildBasePopup(asn: AsForFrontend): string {
    return `
        <div style="font-family: system-ui, -apple-system, sans-serif; min-width: 300px; background: linear-gradient(135deg, #0f172a 0%, #1e293b 100%); border-radius: 16px; padding: 16px; border: 1px solid rgba(71, 85, 105, 0.5); box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);">
            <div style="display: flex; align-items: center; gap: 12px; margin-bottom: 16px; padding-bottom: 12px; border-bottom: 1px solid rgba(71, 85, 105, 0.4);">
                <div style="background: linear-gradient(135deg, #3b82f6, #1d4ed8); padding: 8px 14px; border-radius: 10px; box-shadow: 0 4px 15px rgba(59, 130, 246, 0.3);">
                    <span style="color: white; font-weight: 700; font-size: 15px; letter-spacing: -0.5px;">AS${asn.asn}</span>
                </div>
                <div style="flex: 1; min-width: 0;">
                    <div style="font-weight: 600; color: #f1f5f9; font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">${asn.name}</div>
                    <div style="color: #94a3b8; font-size: 11px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">${asn.organization ?? "—"}</div>
                </div>
            </div>
            <div style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; margin-bottom: 16px;">
                <div style="background: rgba(251, 191, 36, 0.1); padding: 10px 8px; border-radius: 10px; text-align: center; border: 1px solid rgba(251, 191, 36, 0.2);">
                    <div style="color: #fbbf24; font-weight: 700; font-size: 16px;">#${asn.rank}</div>
                    <div style="color: #64748b; font-size: 9px; text-transform: uppercase; letter-spacing: 0.5px; margin-top: 2px;">Rank</div>
                </div>
                <div style="background: rgba(52, 211, 153, 0.1); padding: 10px 8px; border-radius: 10px; text-align: center; border: 1px solid rgba(52, 211, 153, 0.2);">
                    <div style="color: #34d399; font-weight: 700; font-size: 16px;">${asn.prefixes}</div>
                    <div style="color: #64748b; font-size: 9px; text-transform: uppercase; letter-spacing: 0.5px; margin-top: 2px;">Prefixes</div>
                </div>
                <div style="background: rgba(167, 139, 250, 0.1); padding: 10px 8px; border-radius: 10px; text-align: center; border: 1px solid rgba(167, 139, 250, 0.2);">
                    <div style="color: #a78bfa; font-weight: 700; font-size: 16px;">${formatNumber(asn.addresses)}</div>
                    <div style="color: #64748b; font-size: 9px; text-transform: uppercase; letter-spacing: 0.5px; margin-top: 2px;">IPs</div>
                </div>
            </div>
            <div style="display: flex; gap: 8px; flex-wrap: wrap;">
                <a href="/details/${asn.asn}" target="_blank" style="display: inline-flex; align-items: center; gap: 6px; padding: 8px 14px; background: linear-gradient(135deg, #3b82f6, #2563eb); color: white; border-radius: 8px; text-decoration: none; font-size: 12px; font-weight: 600; box-shadow: 0 4px 12px rgba(59, 130, 246, 0.25); transition: all 0.2s;">Details</a>
                <a href="https://bgp.he.net/AS${asn.asn}" target="_blank" style="display: inline-flex; align-items: center; gap: 6px; padding: 8px 14px; background: rgba(71, 85, 105, 0.5); color: #e2e8f0; border-radius: 8px; text-decoration: none; font-size: 12px; font-weight: 500; border: 1px solid rgba(71, 85, 105, 0.5);">BGP.HE</a>
                <a id="shodan-link-${asn.asn}" href="https://www.shodan.io/search?query=asn:AS${asn.asn}" target="_blank" style="display: inline-flex; align-items: center; gap: 6px; padding: 8px 14px; background: rgba(51, 65, 85, 0.5); color: #e2e8f0; border-radius: 8px; font-size: 12px; border: 1px solid rgba(51, 65, 85, 0.5); text-decoration: none;">Shodan</a>
            </div>
            <!--details-start--><!--details-end-->
        </div>
    `;
}

function buildTooltip(asn: AsForFrontend): string {
    return `AS${asn.asn}:${asn.name}:${asn.organization ?? ""}`;
}

function applyDetailsHtml(details: As, baseHtml: string): string {
    let html = baseHtml;
    const degree = details.asrank_data?.degree?.total ?? 0;

    let detailBlock = "<div style=\"margin-top: 16px; padding-top: 16px; border-top: 1px solid rgba(71, 85, 105, 0.4);\">";
    detailBlock += `
        <div style="display: inline-flex; align-items: center; gap: 8px; margin-bottom: 12px;">
            <span style="background: rgba(249, 115, 22, 0.15); color: #fb923c; padding: 6px 12px; border-radius: 8px; font-size: 12px; font-weight: 600; border: 1px solid rgba(249, 115, 22, 0.2);">${degree} connections</span>
        </div>
    `;

    if (details.ipnetdb_data?.ipv4_prefixes?.length) {
        const prefixes = details.ipnetdb_data.ipv4_prefixes;
        const firstPrefix = prefixes[0]?.range ?? "";
        html = html.replace(
            `<a id="shodan-link-${details.asn}" href="https://www.shodan.io/search?query=asn:AS${details.asn}" target="_blank" style="display: inline-flex; align-items: center; gap: 6px; padding: 8px 14px; background: rgba(51, 65, 85, 0.5); color: #e2e8f0; border-radius: 8px; font-size: 12px; border: 1px solid rgba(51, 65, 85, 0.5); text-decoration: none;">Shodan</a>`,
            `<a id="shodan-link-${details.asn}" href="https://www.shodan.io/search?query=net:${firstPrefix}" target="_blank" style="display: inline-flex; align-items: center; gap: 6px; padding: 8px 14px; background: linear-gradient(135deg, #dc2626, #b91c1c); color: white; border-radius: 8px; text-decoration: none; font-size: 12px; font-weight: 600; box-shadow: 0 4px 12px rgba(220, 38, 38, 0.25);">Shodan</a>`
        );

        detailBlock += "<div style=\"margin-top: 12px;\"><div style=\"color: #94a3b8; font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 8px; font-weight: 600;\">Prefixes</div><div style=\"display: flex; flex-wrap: wrap; gap: 6px;\">";
        prefixes.slice(0, 8).forEach((prefix) => {
            const cidr = prefix.range;
            detailBlock += `
                <div style="background: rgba(30, 41, 59, 0.8); padding: 6px 10px; border-radius: 6px; font-size: 11px; border: 1px solid rgba(71, 85, 105, 0.3);">
                    <span style="color: #e2e8f0; font-weight: 500;">${cidr}</span>
                    <a href="https://www.shodan.io/search?query=net%3A${cidr}" target="_blank" style="color: #ef4444; margin-left: 6px; text-decoration: none; font-weight: 600;">S</a>
                    <a href="https://www.zoomeye.org/searchResult?q=cidr%3A${cidr}" target="_blank" style="color: #3b82f6; margin-left: 4px; text-decoration: none; font-weight: 600;">Z</a>
                    <a href="https://search.censys.io/search?resource=hosts&q=ip%3A${cidr}" target="_blank" style="color: #a855f7; margin-left: 4px; text-decoration: none; font-weight: 600;">C</a>
                </div>
            `;
        });
        if (prefixes.length > 8) {
            detailBlock += `<span style="color: #64748b; font-size: 11px; padding: 6px; font-weight: 500;">+${prefixes.length - 8} more</span>`;
        }
        detailBlock += "</div></div>";
    }

    if (details.stanford_asdb?.length) {
        const categories = new Set(details.stanford_asdb.map((c) => c.layer1));
        if (categories.size) {
            detailBlock += "<div style=\"margin-top: 14px;\"><div style=\"color: #94a3b8; font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 8px; font-weight: 600;\">Categories</div><div style=\"display: flex; flex-wrap: wrap; gap: 6px;\">";
            categories.forEach((cat) => {
                detailBlock += `<span style="background: rgba(6, 182, 212, 0.15); color: #22d3ee; padding: 5px 10px; border-radius: 6px; font-size: 11px; font-weight: 500; border: 1px solid rgba(6, 182, 212, 0.2);">${cat}</span>`;
            });
            detailBlock += "</div></div>";
        }
    }

    detailBlock += "</div>";

    const startToken = "<!--details-start-->";
    const endToken = "<!--details-end-->";
    const detailWrapper = `${startToken}${detailBlock}${endToken}`;
    if (html.includes(startToken) && html.includes(endToken)) {
        const blockPattern = new RegExp(`${startToken}[\\s\\S]*?${endToken}`);
        return html.replace(blockPattern, detailWrapper);
    }
    if (html.includes("</div></div></div>")) {
        return html.replace("</div></div></div>", `${detailWrapper}</div></div></div>`);
    }
    return `${html}${detailWrapper}`;
}

function formatWhoisText(whois: WhoIsAsn | null): string {
    if (!whois) {
        return "No WHOIS data available";
    }

    const lines: string[] = [];
    if (whois.as_name) {
        lines.push(`AS Name: ${whois.as_name}`);
    }
    if (whois.descr?.length) {
        lines.push(`Description: ${whois.descr.join(", ")}`);
    }
    if (whois.country) {
        lines.push(`Country: ${whois.country}`);
    }
    if (whois.organisation) {
        lines.push("\nOrganisation: " + whois.organisation.org_name);
        if (whois.organisation.address?.length) {
            lines.push("Address: " + whois.organisation.address.join(", "));
        }
        if (whois.organisation.email) {
            lines.push("Email: " + whois.organisation.email);
        }
    }
    if (whois.contacts?.length) {
        lines.push(`\nContacts (${whois.contacts.length}):`);
        whois.contacts.forEach((c) => {
            lines.push(`  - ${c.name} (${c.nic_hdl})`);
        });
    }
    return lines.join("\n");
}

export default function MapView() {
    const mapContainerRef = useRef<HTMLDivElement | null>(null);
    const mapRef = useRef<L.Map | null>(null);
    const clusterRef = useRef<L.MarkerClusterGroup | null>(null);
    const markersByAsnRef = useRef<Map<number, L.Marker>>(new Map());
    const drawnAsRef = useRef<Map<number, AsForFrontend>>(new Map());
    const detailedAsRef = useRef<Map<number, As>>(new Map());
    const userDataByAsnRef = useRef<Map<number, UserData>>(new Map());
    const whoisLoadingRef = useRef<Set<number>>(new Set());

    const [filters, setFilters] = useState<AsFilters>(DEFAULT_FILTERS);
    const [prevFilters, setPrevFilters] = useState<AsFilters>(DEFAULT_FILTERS);
    const [drawnCount, setDrawnCount] = useState(0);
    const [detailedCount, setDetailedCount] = useState(0);
    const [activeAsn, setActiveAsn] = useState<number | null>(null);
    const [whoisCache, setWhoisCache] = useState<Map<number, string>>(new Map());
    const [listNames, setListNames] = useState<string[]>([]);
    const [listInput, setListInput] = useState("");
    const [activeUserData, setActiveUserData] = useState<UserData | null>(null);
    const [userDataLoading, setUserDataLoading] = useState(false);
    const [commentDraft, setCommentDraft] = useState("");
    const [saveToast, setSaveToast] = useState<string | null>(null);
    const saveToastTimeoutRef = useRef<number | null>(null);

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

    useEffect(() => {
        getListNames()
            .then((names) => setListNames(names))
            .catch((error) => console.error(error));
    }, []);

    useEffect(() => {
        setCommentDraft(activeUserData?.comment ?? "");
    }, [activeUserData, activeAsn]);

    useEffect(() => {
        return () => {
            if (saveToastTimeoutRef.current !== null) {
                window.clearTimeout(saveToastTimeoutRef.current);
            }
        };
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
        const updated = applyDetailsHtml(details, baseHtml);
        popup.setContent(updated);
    }, []);

    const handlePopupOpen = useCallback(
        async (asn: number) => {
            setActiveAsn(asn);

            const cachedUser = userDataByAsnRef.current.get(asn);
            if (cachedUser) {
                setActiveUserData(cachedUser);
            } else {
                setUserDataLoading(true);
                try {
                    const userData = await getUserData(asn);
                    userDataByAsnRef.current.set(asn, userData);
                    setActiveUserData(userData);
                    setListNames((current) =>
                        Array.from(new Set([...current, ...userData.lists])).sort()
                    );
                } catch (error) {
                    console.error(error);
                    setActiveUserData(null);
                } finally {
                    setUserDataLoading(false);
                }
            }

            if (detailedAsRef.current.has(asn)) {
                updateMarkerPopup(asn, detailedAsRef.current.get(asn) as As);
            } else {
                try {
                    const details = await getAsDetails(asn);
                    detailedAsRef.current.set(asn, details);
                    updateMarkerPopup(asn, details);
                    updateCounts();
                } catch (error) {
                    console.error(error);
                }
            }

            if (whoisCache.has(asn) || whoisLoadingRef.current.has(asn)) {
                return;
            }
            whoisLoadingRef.current.add(asn);
            try {
                const whois = await fetchAsWhoisData(asn);
                setWhoisCache((current) => {
                    const next = new Map(current);
                    next.set(asn, formatWhoisText(whois));
                    return next;
                });
            } catch (error) {
                console.error(error);
            } finally {
                whoisLoadingRef.current.delete(asn);
            }
        },
        [updateCounts, updateMarkerPopup, whoisCache]
    );

    const drawAses = useCallback(
        (ases: AsForFrontend[]) => {
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
        },
        [handlePopupOpen, updateCounts]
    );

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
            category: [],
            lists: []
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
        whoisLoadingRef.current.clear();
        setActiveAsn(null);
        setWhoisCache(new Map());
        updateCounts();
    }, [updateCounts]);

    const downloadCsv = useCallback(
        (detailed: boolean) => {
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
        },
        [prevFilters]
    );

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

    const persistUserData = useCallback(
        async (asn: number, lists?: string[] | null, comment?: string | null) => {
            try {
                const updated = await updateUserData(asn, lists, comment);
                userDataByAsnRef.current.set(asn, updated);
                setActiveUserData(updated);
                setListNames((current) =>
                    Array.from(new Set([...current, ...updated.lists])).sort()
                );
                setSaveToast("Saved");
                if (saveToastTimeoutRef.current !== null) {
                    window.clearTimeout(saveToastTimeoutRef.current);
                }
                saveToastTimeoutRef.current = window.setTimeout(() => {
                    setSaveToast(null);
                    saveToastTimeoutRef.current = null;
                }, 1600);
            } catch (error) {
                console.error(error);
            }
        },
        []
    );

    const removeListForActive = useCallback(
        (listName: string) => {
            if (!activeAsn || !activeUserData) {
                return;
            }
            const nextLists = activeUserData.lists.filter((name) => name !== listName);
            persistUserData(activeAsn, nextLists, undefined);
        },
        [activeAsn, activeUserData, persistUserData]
    );

    const addListForActive = useCallback(() => {
        if (!activeAsn || !activeUserData) {
            return;
        }
        const next = listInput.trim();
        if (!next) {
            return;
        }
        const already = activeUserData.lists.includes(next);
        const nextLists = already ? activeUserData.lists : [...activeUserData.lists, next];
        persistUserData(activeAsn, nextLists, undefined);
        setListInput("");
    }, [activeAsn, activeUserData, commentDraft, listInput, persistUserData]);

    const saveCommentForActive = useCallback(() => {
        if (!activeAsn || !activeUserData) {
            return;
        }
        const commentValue = commentDraft.trim();
        const payload = commentValue.length ? commentValue : "";
        persistUserData(activeAsn, undefined, payload);
    }, [activeAsn, activeUserData, commentDraft, persistUserData]);

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

    const whoisText = activeAsn ? whoisCache.get(activeAsn) : undefined;

    return (
        <div className="min-h-screen bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950 text-slate-100 flex flex-col lg:flex-row">
            {saveToast && (
                <div className="fixed right-4 top-4 z-50 px-4 py-2 rounded-xl bg-emerald-500/20 text-emerald-200 border border-emerald-500/30 shadow-lg shadow-emerald-500/20 text-sm font-semibold">
                    {saveToast}
                </div>
            )}
            <div className="flex-1 min-h-[50vh] lg:min-h-screen relative">
                <div ref={mapContainerRef} className="absolute inset-0" />
            </div>

            <div className="w-full lg:w-80 xl:w-96 p-4 lg:p-6 space-y-4 lg:max-h-screen lg:overflow-y-auto bg-slate-900/80 backdrop-blur-xl border-t lg:border-t-0 lg:border-l border-slate-700/50">
                <div className="flex items-center gap-3 mb-2">
                    <div className="p-2 bg-blue-500/20 rounded-xl border border-blue-500/30">
                        <svg className="w-5 h-5 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 20l-5.447-2.724A1 1 0 013 16.382V5.618a1 1 0 011.447-.894L9 7m0 13l6-3m-6 3V7m6 10l4.553 2.276A1 1 0 0021 18.382V7.618a1 1 0 00-.553-.894L15 4m0 13V4m0 0L9 7" />
                        </svg>
                    </div>
                    <div>
                        <h1 className="text-lg font-bold text-white">{"AS Map"}</h1>
                        <p className="text-xs text-slate-400">{"Autonomous Systems Explorer"}</p>
                    </div>
                </div>

                <div className="p-4 rounded-2xl bg-slate-800/40 border border-slate-700/50 backdrop-blur-sm space-y-2">
                    <button
                        onClick={loadFiltered}
                        className="w-full px-4 py-3 bg-gradient-to-r from-blue-600 to-blue-700 hover:from-blue-500 hover:to-blue-600 active:from-blue-700 active:to-blue-800 text-white text-sm font-semibold rounded-xl shadow-lg shadow-blue-500/25 hover:shadow-blue-500/40 transition-all duration-200 flex items-center justify-center gap-2"
                    >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                        </svg>
                        {"Apply Filters"}
                    </button>
                    <button
                        onClick={loadBoundsOnly}
                        className="w-full px-4 py-2.5 bg-slate-700/60 hover:bg-slate-600/60 active:bg-slate-500/60 text-slate-200 text-sm font-medium rounded-xl border border-slate-600/50 transition-all duration-200 hover:border-slate-500/50"
                    >
                        {"Load visible range"}
                    </button>
                    <button
                        onClick={clearMap}
                        className="w-full px-4 py-2.5 bg-red-600/20 hover:bg-red-600/30 active:bg-red-600/40 text-red-400 hover:text-red-300 text-sm font-medium rounded-xl border border-red-600/30 transition-all duration-200 flex items-center justify-center gap-2"
                    >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                        </svg>
                        {"Clear Map"}
                    </button>
                    <div className="grid grid-cols-2 gap-2">
                        <button
                            onClick={() => downloadCsv(false)}
                            className="w-full px-3 py-2 bg-slate-700/40 hover:bg-slate-600/50 active:bg-slate-500/50 text-slate-300 text-xs font-medium rounded-lg border border-slate-600/40 transition-all duration-200 flex items-center justify-center gap-1.5"
                        >
                            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                            </svg>
                            {"CSV"}
                        </button>
                        <button
                            onClick={() => downloadCsv(true)}
                            className="w-full px-3 py-2 bg-slate-700/40 hover:bg-slate-600/50 active:bg-slate-500/50 text-slate-300 text-xs font-medium rounded-lg border border-slate-600/40 transition-all duration-200 flex items-center justify-center gap-1.5"
                        >
                            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                            </svg>
                            {"Detailed"}
                        </button>
                    </div>
                </div>

                <div className="p-4 rounded-2xl bg-slate-800/40 border border-slate-700/50 backdrop-blur-sm">
                    <div className="flex items-center gap-2 mb-4">
                        <svg className="w-4 h-4 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z" />
                        </svg>
                        <span className="text-sm font-semibold text-slate-300">{"Filters"}</span>
                    </div>
                    <div className="space-y-3">
                        <div className="flex items-center gap-3 p-3 rounded-xl bg-slate-700/30 border border-slate-600/30 hover:bg-slate-700/40 transition-colors">
                            <input
                                type="checkbox"
                                id="isBounded"
                                checked={Boolean(filters.bounds)}
                                className="w-4 h-4 bg-slate-800 border-slate-600 rounded text-blue-500 focus:ring-2 focus:ring-blue-500/50 focus:ring-offset-0"
                                onChange={toggleBounded}
                            />
                            <label htmlFor="isBounded" className="text-sm text-slate-300 cursor-pointer select-none">
                                {"Limit to visible area"}
                            </label>
                        </div>

                        <div className="p-3 rounded-xl bg-slate-700/30 border border-slate-600/30">
                            <label className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-3">{"Address Range"}</label>
                            <div className="grid grid-cols-2 gap-3">
                                <div>
                                    <label className="block text-xs text-slate-500 mb-1">{"Min"}</label>
                                    <input
                                        type="number"
                                        id="minAddresses"
                                        value={filters.addresses?.[0] ?? 0}
                                        min={0}
                                        max={99999999}
                                        className="w-full px-3 py-2 bg-slate-800/80 border border-slate-600/50 rounded-lg text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50 transition-all"
                                        onChange={(e) =>
                                            setFilters((current) => ({
                                                ...current,
                                                addresses: [Number(e.target.value), current.addresses?.[1] ?? 0]
                                            }))
                                        }
                                    />
                                </div>
                                <div>
                                    <label className="block text-xs text-slate-500 mb-1">{"Max"}</label>
                                    <input
                                        type="number"
                                        id="maxAddresses"
                                        value={filters.addresses?.[1] ?? 0}
                                        min={0}
                                        max={99999999}
                                        className="w-full px-3 py-2 bg-slate-800/80 border border-slate-600/50 rounded-lg text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50 transition-all"
                                        onChange={(e) =>
                                            setFilters((current) => ({
                                                ...current,
                                                addresses: [current.addresses?.[0] ?? 0, Number(e.target.value)]
                                            }))
                                        }
                                    />
                                </div>
                            </div>
                        </div>

                        <div className="p-3 rounded-xl bg-slate-700/30 border border-slate-600/30">
                            <label className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-3">{"Country"}</label>
                            <div className="flex items-center gap-3">
                                <input
                                    type="text"
                                    id="countryCode"
                                    value={filters.country ?? ""}
                                    maxLength={2}
                                    placeholder="PL"
                                    className="w-20 px-3 py-2 bg-slate-800/80 border border-slate-600/50 rounded-lg text-sm text-slate-200 uppercase focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50 transition-all placeholder-slate-500"
                                    onChange={(e) =>
                                        setFilters((current) => ({
                                            ...current,
                                            country: e.target.value ? e.target.value.toUpperCase() : null
                                        }))
                                    }
                                />
                                <label className="flex items-center gap-2 cursor-pointer select-none">
                                    <input
                                        type="checkbox"
                                        id="excludeCountry"
                                        checked={filters.exclude_country}
                                        className="w-4 h-4 bg-slate-800 border-slate-600 rounded text-red-500 focus:ring-2 focus:ring-red-500/50 focus:ring-offset-0"
                                        onChange={() =>
                                            setFilters((current) => ({
                                                ...current,
                                                exclude_country: !current.exclude_country
                                            }))
                                        }
                                    />
                                    <span className="text-xs text-slate-400">{"Exclude"}</span>
                                </label>
                            </div>
                        </div>

                        <div className="p-3 rounded-xl bg-slate-700/30 border border-slate-600/30">
                            <label className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-3">{"Rank Range"}</label>
                            <div className="grid grid-cols-2 gap-3">
                                <div>
                                    <label className="block text-xs text-slate-500 mb-1">{"Min"}</label>
                                    <input
                                        type="number"
                                        id="minRank"
                                        value={filters.rank?.[0] ?? 0}
                                        min={0}
                                        max={999999}
                                        className="w-full px-3 py-2 bg-slate-800/80 border border-slate-600/50 rounded-lg text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50 transition-all"
                                        onChange={(e) =>
                                            setFilters((current) => ({
                                                ...current,
                                                rank: [Number(e.target.value), current.rank?.[1] ?? 0]
                                            }))
                                        }
                                    />
                                </div>
                                <div>
                                    <label className="block text-xs text-slate-500 mb-1">{"Max"}</label>
                                    <input
                                        type="number"
                                        id="maxRank"
                                        value={filters.rank?.[1] ?? 0}
                                        min={0}
                                        max={999999}
                                        className="w-full px-3 py-2 bg-slate-800/80 border border-slate-600/50 rounded-lg text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50 transition-all"
                                        onChange={(e) =>
                                            setFilters((current) => ({
                                                ...current,
                                                rank: [current.rank?.[0] ?? 0, Number(e.target.value)]
                                            }))
                                        }
                                    />
                                </div>
                            </div>
                        </div>

                        <div className="p-3 rounded-xl bg-slate-700/30 border border-slate-600/30">
                            <label className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-3">{"Organization"}</label>
                            <select
                                id="hasOrg"
                                name="hasOrgSel"
                                className="w-full px-3 py-2 bg-slate-800/80 border border-slate-600/50 rounded-lg text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50 transition-all cursor-pointer"
                                value={filters.has_org === "Yes" ? "yes" : filters.has_org === "No" ? "no" : "both"}
                                onChange={(e) => updateHasOrg(e.target.value)}
                            >
                                <option value="yes">{"Has organization"}</option>
                                <option value="no">{"No organization"}</option>
                                <option value="both">{"Both"}</option>
                            </select>
                        </div>

                        <div className="p-3 rounded-xl bg-slate-700/30 border border-slate-600/30">
                            <label className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-3">{"Category"}</label>
                            <select
                                id="category"
                                name="category"
                                multiple
                                className="w-full h-32 px-3 py-2 bg-slate-800/80 border border-slate-600/50 rounded-lg text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50 transition-all"
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

                        <div className="p-3 rounded-xl bg-slate-700/30 border border-slate-600/30">
                            <label className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-3">{"Lists"}</label>
                            {listNames.length ? (
                                <select
                                    id="lists"
                                    name="lists"
                                    multiple
                                    className="w-full h-24 px-3 py-2 bg-slate-800/80 border border-slate-600/50 rounded-lg text-sm text-slate-200 focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50 transition-all"
                                    value={filters.lists}
                                    onChange={(e) => {
                                        const selected = Array.from(e.target.selectedOptions).map((o) => o.value);
                                        setFilters((current) => ({ ...current, lists: selected }));
                                    }}
                                >
                                    {listNames.map((name) => (
                                        <option key={name} value={name}>
                                            {name}
                                        </option>
                                    ))}
                                </select>
                            ) : (
                                <p className="text-xs text-slate-500">{"No lists yet"}</p>
                            )}
                        </div>
                    </div>
                </div>

                {activeAsn && (
                    <div className="p-4 rounded-2xl bg-slate-800/40 border border-slate-700/50 backdrop-blur-sm space-y-3">
                        <div className="flex items-center justify-between">
                            <div className="flex items-center gap-2">
                                <svg className="w-4 h-4 text-amber-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M5 13l4 4L19 7" />
                                </svg>
                                <span className="text-sm font-semibold text-slate-300">{`Favorites for AS${activeAsn}`}</span>
                            </div>
                            {userDataLoading && (
                                <span className="text-xs text-slate-500">{"Loading..."}</span>
                            )}
                        </div>

                        {activeUserData ? (
                            <div className="space-y-3">
                                <div className="space-y-2">
                                    {activeUserData.lists.length ? (
                                        <div className="space-y-2">
                                            {activeUserData.lists.map((name) => (
                                                <div
                                                    key={name}
                                                    className="flex items-center justify-between gap-2 rounded-lg border border-slate-700/50 bg-slate-900/40 px-2 py-1.5 text-xs text-slate-300"
                                                >
                                                    <span className="truncate">{name}</span>
                                                    <button
                                                        onClick={() => removeListForActive(name)}
                                                        className="p-1 rounded-md text-slate-400 hover:text-red-300 hover:bg-red-500/10 border border-transparent hover:border-red-500/20 transition"
                                                        aria-label={`Remove ${name}`}
                                                        title="Remove list"
                                                    >
                                                        <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M6 18L18 6M6 6l12 12" />
                                                        </svg>
                                                    </button>
                                                </div>
                                            ))}
                                        </div>
                                    ) : (
                                        <p className="text-xs text-slate-500">{"No lists yet"}</p>
                                    )}
                                </div>

                                <div className="flex gap-2">
                                    <input
                                        type="text"
                                        value={listInput}
                                        placeholder="New list name"
                                        className="flex-1 px-3 py-2 bg-slate-900/70 border border-slate-700/50 rounded-lg text-xs text-slate-200 focus:outline-none focus:ring-2 focus:ring-amber-400/40 focus:border-amber-400/40"
                                        onChange={(e) => setListInput(e.target.value)}
                                    />
                                    <button
                                        onClick={addListForActive}
                                        className="px-3 py-2 text-xs font-semibold rounded-lg bg-amber-500/20 text-amber-200 border border-amber-500/30 hover:bg-amber-500/30 transition"
                                    >
                                        {"Add"}
                                    </button>
                                </div>

                                <div>
                                    <label className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">{"Comment"}</label>
                                    <textarea
                                        value={commentDraft}
                                        rows={3}
                                        className="w-full px-3 py-2 bg-slate-900/70 border border-slate-700/50 rounded-lg text-xs text-slate-200 focus:outline-none focus:ring-2 focus:ring-amber-400/40 focus:border-amber-400/40"
                                        onChange={(e) => setCommentDraft(e.target.value)}
                                    />
                                    <button
                                        onClick={saveCommentForActive}
                                        className="mt-2 w-full px-3 py-2 text-xs font-semibold rounded-lg bg-slate-700/50 text-slate-200 border border-slate-600/40 hover:bg-slate-600/60 transition"
                                    >
                                        {"Save comment"}
                                    </button>
                                </div>
                            </div>
                        ) : (
                            <p className="text-xs text-slate-500">{"No user data loaded"}</p>
                        )}
                    </div>
                )}

                <div className="p-4 rounded-2xl bg-slate-800/40 border border-slate-700/50 backdrop-blur-sm">
                    <div className="flex items-center gap-2 mb-3">
                        <svg className="w-4 h-4 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
                        </svg>
                        <span className="text-sm font-semibold text-slate-300">{"Statistics"}</span>
                    </div>
                    <div className="grid grid-cols-2 gap-3">
                        <div className="p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/20">
                            <p className="text-2xl font-bold text-emerald-400 tabular-nums">{drawnCount}</p>
                            <p className="text-xs text-slate-400">{"Drawn"}</p>
                        </div>
                        <div className="p-3 rounded-xl bg-purple-500/10 border border-purple-500/20">
                            <p className="text-2xl font-bold text-purple-400 tabular-nums">{detailedCount}</p>
                            <p className="text-xs text-slate-400">{"Detailed"}</p>
                        </div>
                    </div>
                </div>

                {whoisText && (
                    <div className="p-4 rounded-2xl bg-slate-800/40 border border-slate-700/50 backdrop-blur-sm">
                        <div className="flex items-center gap-2 mb-3">
                            <svg className="w-4 h-4 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                            </svg>
                            <span className="text-sm font-semibold text-slate-300">{`WHOIS AS${activeAsn}`}</span>
                        </div>
                        <pre className="text-xs text-slate-400 whitespace-pre-wrap max-h-48 overflow-auto p-3 rounded-lg bg-slate-900/50 border border-slate-700/30">
                            {whoisText}
                        </pre>
                    </div>
                )}
            </div>
        </div>
    );
}
