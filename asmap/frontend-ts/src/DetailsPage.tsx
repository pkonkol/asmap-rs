import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link, useParams } from "react-router-dom";
import countries from "i18n-iso-countries";
import en from "i18n-iso-countries/langs/en.json";
import type {
    As,
    AsrankAsn,
    Coord,
    GeocodedAddress,
    IPNetDBAsn,
    StanfordASdbCategory,
    UserData,
    WhoIsAsn
} from "./protocol/types";
import {
    fetchAsWhoisData,
    getAsDetails,
    getListNames,
    getUserData,
    saveGeocoding,
    updateUserData
} from "./api/ws";

countries.registerLocale(en);

interface AddressComponents {
    street?: string;
    city?: string;
    postalcode?: string;
    country?: string;
}

const POLISH_LETTERS = "A-Za-zżźćńółęąśŻŹĆĄŚĘŁÓŃ";

function formatNumber(n: number): string {
    if (n >= 1_000_000_000) {
        return `${(n / 1_000_000_000).toFixed(1)}B`;
    }
    if (n >= 1_000_000) {
        return `${(n / 1_000_000).toFixed(1)}M`;
    }
    if (n >= 1_000) {
        return `${(n / 1_000).toFixed(1)}K`;
    }
    return n.toString();
}

function countryFlag(code: string): string {
    if (code.length !== 2) {
        return "🌐";
    }
    const upper = code.toUpperCase();
    const chars = [...upper];
    const base = 0x1f1e6 - "A".charCodeAt(0);
    const flag = chars
        .map((c) => String.fromCodePoint(base + c.charCodeAt(0)))
        .join("");
    return flag.length === 2 ? flag : "🌐";
}

function categoryColor(layer1: string): string {
    const key = layer1.toLowerCase();
    if (key.includes("isp") || key.includes("transit")) {
        return "bg-blue-500/15 border-blue-500/25 text-blue-200";
    }
    if (key.includes("enterprise") || key.includes("business")) {
        return "bg-emerald-500/15 border-emerald-500/25 text-emerald-200";
    }
    if (key.includes("education") || key.includes("research")) {
        return "bg-violet-500/15 border-violet-500/25 text-violet-200";
    }
    if (key.includes("government")) {
        return "bg-amber-500/15 border-amber-500/25 text-amber-200";
    }
    if (key.includes("content") || key.includes("cdn")) {
        return "bg-pink-500/15 border-pink-500/25 text-pink-200";
    }
    if (key.includes("hosting") || key.includes("cloud")) {
        return "bg-cyan-500/15 border-cyan-500/25 text-cyan-200";
    }
    return "bg-slate-700/35 border-slate-600/40 text-slate-200";
}

function parseAddress(address: string): AddressComponents {
    let cleaned = address;
    const orgPatterns = [
        /^(?:[^,]+,\s*)(ul\.)/i,
        /^(?:[^,]+,\s*)(al\.)/i,
        /^(?:[^,]+,\s*)(pl\.)/i,
        /^(?:[^,]+,\s*)(trakt)/i
    ];

    orgPatterns.forEach((pattern) => {
        const match = cleaned.match(pattern);
        if (match) {
            cleaned = cleaned.replace(pattern, match[1]);
        }
    });

    cleaned = cleaned
        .replace(new RegExp(`\\bul\\.([${POLISH_LETTERS}])`, "gi"), "ul. $1")
        .replace(new RegExp(`\\bal\\.([${POLISH_LETTERS}])`, "gi"), "al. $1")
        .replace(new RegExp(`\\bpl\\.([${POLISH_LETTERS}])`, "gi"), "pl. $1");

    const components: AddressComponents = {};
    const postalMatch = cleaned.match(/(\d{2}-?\d{3})/);
    if (postalMatch) {
        components.postalcode = postalMatch[1];
    }

    const countryMatch = cleaned.match(/,\s*([A-Za-z]+)\s*$/);
    if (countryMatch) {
        components.country = countryMatch[1];
    }

    const cityMatch = cleaned.match(new RegExp(`\\d{2}-?\\d{3}\\s+([${POLISH_LETTERS}]+)`));
    if (cityMatch) {
        components.city = cityMatch[1];
    }

    if (!components.city) {
        const parts = cleaned.split(",").map((p) => p.trim());
        if (parts.length >= 2) {
            const maybeCity = parts[parts.length - 2];
            const cityFallback = maybeCity.match(new RegExp(`([${POLISH_LETTERS}]{3,})`));
            if (cityFallback) {
                components.city = cityFallback[1];
            }
        }
    }

    const streetMatch = cleaned.match(
        new RegExp(
            `((?:ul\\.|ulica|al\\.|aleja|pl\\.|plac|trakt)\\s*[${POLISH_LETTERS}\\.]+(?:\\s+[${POLISH_LETTERS}\\.]+)*\\s*\\d*(?:/\\d+)?)`,
            "i"
        )
    );
    if (streetMatch) {
        components.street = normalizeStreet(streetMatch[1]);
    }

    if (!components.street) {
        const parts = cleaned.split(",").map((p) => p.trim());
        if (parts.length) {
            if (/\d/.test(parts[0])) {
                components.street = normalizeStreet(parts[0]);
            }
        }
    }

    return components;
}

function normalizeStreet(street: string): string {
    let result = street;
    result = result.replace(/^(ul\.|ulica)\s*/i, "");
    result = result.replace(/^aleja\s+/i, "al. ");
    result = result.replace(/^plac\s+/i, "pl. ");
    result = result.replace(/(\d+)\/\d+\s*$/, "$1");
    return result.trim();
}

function hasValidComponents(components: AddressComponents): boolean {
    return Boolean(components.city || components.street);
}

function componentsToQuery(components: AddressComponents): string {
    const params = new URLSearchParams();
    if (components.street) params.set("street", components.street);
    if (components.city) params.set("city", components.city);
    if (components.postalcode) params.set("postalcode", components.postalcode);
    if (components.country) params.set("country", components.country);
    return params.toString();
}

async function geocodeStructured(components: AddressComponents): Promise<{ coord: Coord; display: string }> {
    const query = componentsToQuery(components);
    const url = `https://nominatim.openstreetmap.org/search?${query}&format=json&limit=1`;
    const response = await fetch(url, { headers: { "Accept-Language": "en" } });
    if (!response.ok) {
        if (response.status === 429) {
            throw new Error("Rate limit exceeded");
        }
        throw new Error(`HTTP error: ${response.status}`);
    }
    const results: Array<{ lat: string; lon: string; display_name: string }> = await response.json();
    if (!results.length) {
        throw new Error("No results found");
    }
    const first = results[0];
    return {
        coord: { lat: Number(first.lat), lon: Number(first.lon) },
        display: first.display_name
    };
}

async function geocodeAddress(address: string): Promise<GeocodedAddress> {
    const components = parseAddress(address);
    if (!hasValidComponents(components)) {
        return {
            original_address: address,
            normalized_address: address,
            coordinate: null,
            display_name: null,
            error: "Unable to parse address"
        };
    }

    try {
        const { coord, display } = await geocodeStructured(components);
        return {
            original_address: address,
            normalized_address: componentsToQuery(components),
            coordinate: coord,
            display_name: display,
            error: null
        };
    } catch (error) {
        if (components.city) {
            const fallback: AddressComponents = { city: components.city, country: components.country };
            try {
                const { coord, display } = await geocodeStructured(fallback);
                return {
                    original_address: address,
                    normalized_address: componentsToQuery(fallback),
                    coordinate: coord,
                    display_name: display,
                    error: null
                };
            } catch (fallbackError) {
                return {
                    original_address: address,
                    normalized_address: componentsToQuery(fallback),
                    coordinate: null,
                    display_name: null,
                    error: fallbackError instanceof Error ? fallbackError.message : String(fallbackError)
                };
            }
        }

        return {
            original_address: address,
            normalized_address: componentsToQuery(components),
            coordinate: null,
            display_name: null,
            error: error instanceof Error ? error.message : String(error)
        };
    }
}

function collectWhoisAddresses(whois: WhoIsAsn | null): string[] {
    if (!whois) {
        return [];
    }

    const addresses: string[] = [];
    if (whois.organisation?.address?.length) {
        const full = whois.organisation.address.map((line) => line.trim()).filter(Boolean).join(", ");
        if (full) {
            addresses.push(full);
        }
    }

    whois.contacts?.forEach((contact) => {
        if (contact.address?.length) {
            const full = contact.address.map((line) => line.trim()).filter(Boolean).join(", ");
            if (full) {
                addresses.push(full);
            }
        }
    });

    const seen = new Set<string>();
    return addresses.filter((addr) => (seen.has(addr) ? false : seen.add(addr)));
}

export default function DetailsPage() {
    const { id } = useParams();
    const asn = useMemo(() => Number(id), [id]);

    const [asDetails, setAsDetails] = useState<As | null>(null);
    const [whoisData, setWhoisData] = useState<WhoIsAsn | null>(null);
    const [userData, setUserData] = useState<UserData | null>(null);
    const [userDataLoading, setUserDataLoading] = useState(true);
    const [listNames, setListNames] = useState<string[]>([]);
    const [listInput, setListInput] = useState("");
    const [commentDraft, setCommentDraft] = useState("");
    const [saveToast, setSaveToast] = useState<string | null>(null);
    const saveToastTimeoutRef = useRef<number | null>(null);
    const [loading, setLoading] = useState(true);
    const [whoisLoading, setWhoisLoading] = useState(true);
    const [geocoding, setGeocoding] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [geocodedAddresses, setGeocodedAddresses] = useState<GeocodedAddress[]>([]);

    useEffect(() => {
        if (!Number.isFinite(asn)) {
            setError("Invalid ASN");
            setLoading(false);
            return;
        }

        setLoading(true);
        setWhoisLoading(true);
        setUserDataLoading(true);
        setError(null);
        setAsDetails(null);
        setWhoisData(null);
        setUserData(null);
        setGeocodedAddresses([]);

        getAsDetails(asn)
            .then((details) => {
                setAsDetails(details);
                setLoading(false);
            })
            .catch((err) => {
                setError(`Failed to load AS details: ${err}`);
                setLoading(false);
            });

        fetchAsWhoisData(asn)
            .then((whois) => {
                setWhoisData(whois ?? null);
                setWhoisLoading(false);
            })
            .catch(() => {
                setWhoisData(null);
                setWhoisLoading(false);
            });

        getUserData(asn)
            .then((data) => {
                setUserData(data);
                if (data.geocoded_addresses?.length) {
                    setGeocodedAddresses(data.geocoded_addresses);
                }
                setUserDataLoading(false);
            })
            .catch(() => {
                setUserData(null);
                setUserDataLoading(false);
            });
    }, [asn]);

    useEffect(() => {
        getListNames()
            .then((names) => setListNames(names))
            .catch((error) => console.error(error));
    }, []);

    useEffect(() => {
        setCommentDraft(userData?.comment ?? "");
        if (userData?.lists?.length) {
            setListNames((current) => Array.from(new Set([...current, ...userData.lists])).sort());
        }
    }, [userData]);

    useEffect(() => {
        return () => {
            if (saveToastTimeoutRef.current !== null) {
                window.clearTimeout(saveToastTimeoutRef.current);
            }
        };
    }, []);

    const onGeocode = useCallback(async () => {
        if (geocoding || !whoisData) {
            return;
        }
        const addresses = collectWhoisAddresses(whoisData);
        if (!addresses.length) {
            return;
        }

        setGeocoding(true);
        const results: GeocodedAddress[] = [];
        for (let i = 0; i < addresses.length; i += 1) {
            const address = addresses[i];
            // Throttle to avoid hitting rate limits
            if (i > 0) {
                await new Promise((resolve) => setTimeout(resolve, 1100));
            }
            const result = await geocodeAddress(address);
            results.push(result);
        }
        setGeocodedAddresses(results);
        try {
            const updated = await saveGeocoding(asn, results);
            setUserData(updated);
        } catch (error) {
            console.error(error);
        }
        setGeocoding(false);
    }, [asn, geocoding, whoisData]);

    const persistUserData = useCallback(
        async (lists?: string[] | null, comment?: string | null) => {
            try {
                const updated = await updateUserData(asn, lists, comment);
                setUserData(updated);
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
        [asn]
    );

    const removeList = useCallback(
        (name: string) => {
            if (!userData) {
                return;
            }
            const nextLists = userData.lists.filter((list) => list !== name);
            persistUserData(nextLists, undefined);
        },
        [persistUserData, userData]
    );

    const addList = useCallback(() => {
        if (!userData) {
            return;
        }
        const next = listInput.trim();
        if (!next) {
            return;
        }
        const nextLists = userData.lists.includes(next)
            ? userData.lists
            : [...userData.lists, next];
        persistUserData(nextLists, undefined);
        setListInput("");
    }, [commentDraft, listInput, persistUserData, userData]);

    const saveComment = useCallback(() => {
        if (!userData) {
            return;
        }
        const commentValue = commentDraft.trim();
        const payload = commentValue.length ? commentValue : "";
        persistUserData(undefined, payload);
    }, [commentDraft, persistUserData, userData]);

    if (error) {
        return (
            <div className="min-h-screen bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950 text-slate-100">
                <div className="flex items-center justify-center min-h-[60vh]">
                    <div className="max-w-md w-full p-7 sm:p-8 rounded-3xl bg-red-950/40 border border-red-800/40 backdrop-blur-sm shadow-[0_18px_60px_-35px_rgba(0,0,0,0.9)]">
                        <div className="flex items-start gap-4">
                            <div className="p-3 bg-red-500/15 rounded-2xl border border-red-500/20">
                                <svg className="w-6 h-6 text-red-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                                </svg>
                            </div>
                            <div className="min-w-0">
                                <h3 className="text-lg font-semibold text-red-200 tracking-tight">{"Error Loading Data"}</h3>
                                <p className="text-red-300/80 text-sm mt-1 break-words">{error}</p>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        );
    }

    if (loading) {
        return (
            <div className="min-h-screen bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950 text-slate-100">
                <div className="flex items-center justify-center min-h-[60vh]">
                    <div className="text-center">
                        <div className="relative w-16 h-16 mx-auto mb-6">
                            <div className="absolute inset-0 rounded-full border-4 border-slate-700/70"></div>
                            <div className="absolute inset-0 rounded-full border-4 border-blue-500 border-t-transparent animate-spin"></div>
                        </div>
                        <p className="text-slate-300 font-medium tracking-tight">{"Loading AS details..."}</p>
                        <p className="text-slate-500 text-sm mt-1">{"Fetching core dataset + WHOIS in background"}</p>
                    </div>
                </div>
            </div>
        );
    }

    if (!asDetails) {
        return (
            <div className="min-h-screen bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950 text-slate-100">
                <div className="flex items-center justify-center min-h-[60vh]">
                    <div className="text-center p-7 sm:p-8 rounded-3xl bg-slate-900/40 border border-slate-800/60 backdrop-blur-sm shadow-[0_18px_60px_-35px_rgba(0,0,0,0.9)] max-w-md">
                        <div className="p-4 bg-slate-800/40 border border-slate-700/40 rounded-2xl w-fit mx-auto mb-4">
                            <svg className="w-8 h-8 text-slate-300/70" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                            </svg>
                        </div>
                        <h3 className="text-xl font-semibold text-slate-100 mb-2 tracking-tight">{"AS Not Found"}</h3>
                        <p className="text-slate-400 text-sm leading-relaxed">{"The requested autonomous system could not be found."}</p>
                    </div>
                </div>
            </div>
        );
    }

    const asrank = asDetails.asrank_data as AsrankAsn | null;
    const ipnetdb = asDetails.ipnetdb_data as IPNetDBAsn | null;
    const countryCode = asrank?.country_iso ?? "??";
    const countryName = countries.getName(countryCode, "en") ?? countryCode;

    const whoisHasAddresses = Boolean(
        whoisData?.organisation?.address?.length || whoisData?.contacts?.some((c) => c.address?.length)
    );

    const successful = geocodedAddresses.filter((addr) => addr.coordinate);
    const failed = geocodedAddresses.filter((addr) => !addr.coordinate);

    const stats = asrank
        ? [
            { label: "Global Rank", value: `#${asrank.rank}`, icon: "chart-bar", gradient: "from-blue-500 to-cyan-500" },
            { label: "Prefixes", value: String(asrank.prefixes), icon: "globe-alt", gradient: "from-emerald-500 to-teal-500" },
            { label: "IP Addresses", value: formatNumber(asrank.addresses), icon: "server", gradient: "from-purple-500 to-pink-500" },
            { label: "Connections", value: String(asrank.degree.total), icon: "share", gradient: "from-orange-500 to-amber-500" }
        ]
        : [];

    const externalLinks = [
        { name: "BGP Hurricane Electric", url: `https://bgp.he.net/AS${asDetails.asn}`, gradient: "from-blue-600 to-blue-700" },
        { name: "RIPE Stat", url: `https://stat.ripe.net/AS${asDetails.asn}`, gradient: "from-amber-600 to-orange-700" }
    ];

    return (
        <div className="min-h-screen bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950 text-slate-100">
            {saveToast && (
                <div className="fixed right-4 top-4 z-50 px-4 py-2 rounded-xl bg-emerald-500/20 text-emerald-200 border border-emerald-500/30 shadow-lg shadow-emerald-500/20 text-sm font-semibold">
                    {saveToast}
                </div>
            )}
            <nav className="sticky top-0 z-50 backdrop-blur-xl bg-slate-950/60 border-b border-slate-800/70">
                <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-4">
                    <div className="flex items-center justify-between gap-4">
                        <Link
                            to="/"
                            className="group inline-flex items-center gap-3 text-slate-300 hover:text-white transition-colors duration-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/70 focus-visible:ring-offset-2 focus-visible:ring-offset-slate-950 rounded-xl"
                        >
                            <div className="p-2 rounded-xl bg-slate-800/50 border border-slate-700/50 group-hover:bg-blue-600/15 group-hover:border-blue-500/30 transition-colors">
                                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
                                </svg>
                            </div>
                            <span className="font-medium tracking-tight">{"Back to Map"}</span>
                        </Link>
                        {asDetails && (
                            <div className="flex items-center gap-2">
                                <span className="px-3 py-1 text-xs font-semibold bg-emerald-500/15 text-emerald-300 rounded-full border border-emerald-500/25">
                                    {"ACTIVE"}
                                </span>
                            </div>
                        )}
                    </div>
                </div>
            </nav>

            <main className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
                <div className="space-y-8">
                    <div className="relative overflow-hidden rounded-3xl bg-gradient-to-r from-blue-600/20 via-purple-600/15 to-cyan-600/20 border border-slate-700/60 shadow-[0_30px_90px_-60px_rgba(0,0,0,0.95)]">
                        <div className="absolute inset-0 overflow-hidden pointer-events-none">
                            <div className="absolute -top-28 -right-28 w-[34rem] h-[34rem] bg-blue-500/10 rounded-full blur-3xl"></div>
                            <div className="absolute -bottom-28 -left-28 w-[34rem] h-[34rem] bg-purple-500/10 rounded-full blur-3xl"></div>
                            <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_top,rgba(59,130,246,0.08),transparent_55%)]"></div>
                        </div>
                        <div className="relative p-6 sm:p-8 md:p-10">
                            <div className="flex flex-col md:flex-row md:items-start md:justify-between gap-6">
                                <div className="flex-1 min-w-0">
                                    <div className="inline-flex items-center gap-2 px-4 py-2 bg-slate-950/35 backdrop-blur rounded-2xl border border-slate-600/45 shadow-sm mb-4">
                                        <div className="w-2 h-2 bg-emerald-400 rounded-full animate-pulse"></div>
                                        <span className="text-sm font-mono font-bold text-white">{`AS${asDetails.asn}`}</span>
                                    </div>
                                    {asrank ? (
                                        <>
                                            <h1 className="text-3xl sm:text-4xl md:text-5xl font-bold text-white mb-3 leading-[1.1] tracking-tight break-words">
                                                {asrank.name}
                                            </h1>
                                            {asrank.organization && (
                                                <p className="text-base md:text-lg text-slate-300 flex items-start gap-2 leading-snug">
                                                    <svg className="w-5 h-5 text-slate-400 mt-0.5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4" />
                                                    </svg>
                                                    <span className="break-words">{asrank.organization}</span>
                                                </p>
                                            )}
                                        </>
                                    ) : (
                                        <>
                                            <h1 className="text-3xl sm:text-4xl md:text-5xl font-bold text-white leading-[1.1] tracking-tight">
                                                {`AS${asDetails.asn}`}
                                            </h1>
                                            <p className="text-slate-400 text-sm mt-2">{"No ASRank metadata available."}</p>
                                        </>
                                    )}
                                </div>
                                <div className="flex-shrink-0">
                                    <div className="flex items-center gap-3 px-5 py-3 bg-slate-950/30 backdrop-blur rounded-2xl border border-slate-600/35 shadow-sm">
                                        <span className="text-4xl">{countryFlag(countryCode)}</span>
                                        <div>
                                            <p className="text-[11px] text-slate-400 uppercase tracking-wider">{"Country"}</p>
                                            <p className="text-lg font-semibold text-white tracking-tight">{countryName}</p>
                                        </div>
                                    </div>
                                </div>
                            </div>
                            <div className="mt-8 h-px bg-gradient-to-r from-transparent via-slate-700/60 to-transparent"></div>
                        </div>
                    </div>

                    {stats.length > 0 && (
                        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                            {stats.map((stat) => (
                                <div
                                    key={stat.label}
                                    className="group relative p-5 rounded-2xl bg-slate-900/40 border border-slate-800/60 backdrop-blur-sm shadow-[0_10px_40px_-25px_rgba(0,0,0,0.85)] transition-all duration-300 hover:border-slate-700/70 hover:shadow-[0_18px_60px_-35px_rgba(0,0,0,0.9)] hover:-translate-y-[1px]"
                                >
                                    <div className={`absolute inset-0 bg-gradient-to-br ${stat.gradient} opacity-0 group-hover:opacity-[0.06] rounded-2xl transition-opacity`}></div>
                                    <div className="relative">
                                        <div className={`w-10 h-10 mb-3 rounded-xl bg-gradient-to-br ${stat.gradient} flex items-center justify-center shadow-sm`}>
                                            {stat.icon === "chart-bar" && (
                                                <svg className="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
                                                </svg>
                                            )}
                                            {stat.icon === "globe-alt" && (
                                                <svg className="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
                                                </svg>
                                            )}
                                            {stat.icon === "server" && (
                                                <svg className="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
                                                </svg>
                                            )}
                                            {stat.icon === "share" && (
                                                <svg className="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z" />
                                                </svg>
                                            )}
                                        </div>
                                        <p className="text-2xl font-bold text-white mb-1 tracking-tight tabular-nums">{stat.value}</p>
                                        <p className="text-sm text-slate-400">{stat.label}</p>
                                    </div>
                                </div>
                            ))}
                        </div>
                    )}

                    <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
                        <div className="lg:col-span-2 space-y-6">
                            {ipnetdb && ipnetdb.ipv4_prefixes.length > 0 && (
                                <div className="p-6 rounded-2xl bg-slate-900/40 border border-slate-800/60 backdrop-blur-sm shadow-[0_10px_40px_-25px_rgba(0,0,0,0.85)] transition-all duration-300 hover:border-slate-700/70 hover:shadow-[0_18px_60px_-35px_rgba(0,0,0,0.9)]">
                                    <div className="flex items-start gap-3 mb-5">
                                        <div className="p-2 bg-emerald-500/15 rounded-xl border border-emerald-500/20">
                                            <svg className="w-5 h-5 text-emerald-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
                                            </svg>
                                        </div>
                                        <div className="min-w-0">
                                            <h3 className="text-lg font-semibold text-white tracking-tight">{"IPv4 Prefixes"}</h3>
                                            <p className="text-sm text-slate-400">{`${ipnetdb.ipv4_prefixes.length} announced prefixes`}</p>
                                        </div>
                                    </div>

                                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 max-h-80 overflow-y-auto pr-2 [scrollbar-width:thin] [scrollbar-color:rgba(148,163,184,0.55)_rgba(15,23,42,0.35)] [&::-webkit-scrollbar]:w-2.5 [&::-webkit-scrollbar-track]:bg-slate-950/30 [&::-webkit-scrollbar-track]:rounded-full [&::-webkit-scrollbar-thumb]:bg-slate-400/40 [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb:hover]:bg-slate-300/50">
                                        {ipnetdb.ipv4_prefixes.map((prefix) => (
                                            <div
                                                key={prefix.range}
                                                className="group flex items-center justify-between gap-3 p-3 rounded-xl bg-slate-800/35 border border-slate-700/40 hover:bg-slate-800/55 hover:border-slate-600/60 transition-all duration-200"
                                            >
                                                <code className="font-mono text-sm text-emerald-200 font-semibold tracking-tight break-all">
                                                    {prefix.range}
                                                </code>

                                                <div className="flex items-center gap-1 opacity-100 sm:opacity-0 sm:group-hover:opacity-100 transition-opacity">
                                                    <a
                                                        href={`https://www.shodan.io/search?query=net%3A${prefix.range}`}
                                                        target="_blank"
                                                        className="p-1.5 rounded-lg hover:bg-orange-500/15 border border-transparent hover:border-orange-500/25 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-orange-400/60"
                                                        title="Search on Shodan"
                                                    >
                                                        <span className="text-xs font-bold text-orange-300">{"S"}</span>
                                                    </a>
                                                    <a
                                                        href={`https://www.zoomeye.org/searchResult?q=cidr%3A${prefix.range}`}
                                                        target="_blank"
                                                        className="p-1.5 rounded-lg hover:bg-blue-500/15 border border-transparent hover:border-blue-500/25 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-400/60"
                                                        title="Search on ZoomEye"
                                                    >
                                                        <span className="text-xs font-bold text-blue-300">{"Z"}</span>
                                                    </a>
                                                    <a
                                                        href={`https://search.censys.io/search?resource=hosts&q=ip%3A${prefix.range}`}
                                                        target="_blank"
                                                        className="p-1.5 rounded-lg hover:bg-purple-500/15 border border-transparent hover:border-purple-500/25 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-400/60"
                                                        title="Search on Censys"
                                                    >
                                                        <span className="text-xs font-bold text-purple-300">{"C"}</span>
                                                    </a>
                                                </div>
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            )}

                            {asDetails.stanford_asdb.length > 0 && (
                                <div className="p-6 rounded-2xl bg-slate-900/40 border border-slate-800/60 backdrop-blur-sm shadow-[0_10px_40px_-25px_rgba(0,0,0,0.85)] transition-all duration-300 hover:border-slate-700/70 hover:shadow-[0_18px_60px_-35px_rgba(0,0,0,0.9)]">
                                    <div className="flex items-start gap-3 mb-5">
                                        <div className="p-2 bg-violet-500/15 rounded-xl border border-violet-500/20">
                                            <svg className="w-5 h-5 text-violet-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z" />
                                            </svg>
                                        </div>
                                        <div className="min-w-0">
                                            <h3 className="text-lg font-semibold text-white tracking-tight">{"Classifications"}</h3>
                                            <p className="text-sm text-slate-400">{"Stanford ASDB categories"}</p>
                                        </div>
                                    </div>

                                    <div className="flex flex-wrap gap-2">
                                        {asDetails.stanford_asdb.map((cat: StanfordASdbCategory, index: number) => (
                                            <div
                                                key={`${cat.layer1}-${cat.layer2}-${index}`}
                                                className={`px-4 py-2 rounded-xl border transition-all duration-200 hover:scale-[1.03] hover:-translate-y-[1px] shadow-sm ${categoryColor(cat.layer1)}`}
                                            >
                                                <span className="font-semibold tracking-tight">{cat.layer1}</span>
                                                {cat.layer2 && (
                                                    <>
                                                        <span className="text-slate-400 mx-1">{"›"}</span>
                                                        <span className="opacity-85">{cat.layer2}</span>
                                                    </>
                                                )}
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            )}

                            <div className="p-6 rounded-2xl bg-slate-900/40 border border-slate-800/60 backdrop-blur-sm shadow-[0_10px_40px_-25px_rgba(0,0,0,0.85)] transition-all duration-300 hover:border-slate-700/70 hover:shadow-[0_18px_60px_-35px_rgba(0,0,0,0.9)]">
                                <div className="flex items-start justify-between gap-3 mb-5">
                                    <div className="flex items-start gap-3">
                                        <div className="p-2 bg-amber-500/15 rounded-xl border border-amber-500/20">
                                            <svg className="w-5 h-5 text-amber-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                            </svg>
                                        </div>
                                        <div className="min-w-0">
                                            <h3 className="text-lg font-semibold text-white tracking-tight">{"WHOIS Information"}</h3>
                                            <p className="text-sm text-slate-400">{"Registry data from RIPE"}</p>
                                        </div>
                                    </div>
                                    {userDataLoading && (
                                        <span className="text-xs text-slate-500">{"Loading saved data..."}</span>
                                    )}
                                    {whoisHasAddresses && !whoisLoading && (
                                        <button
                                            onClick={onGeocode}
                                            disabled={geocoding}
                                            className={`inline-flex items-center gap-2 px-4 py-2 rounded-xl text-sm font-medium transition-all duration-200 ${geocoding
                                                ? "bg-slate-700/50 text-slate-400 cursor-not-allowed"
                                                : "bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 hover:bg-emerald-500/30 hover:border-emerald-400/40"
                                                }`}
                                        >
                                            {geocoding ? (
                                                <>
                                                    <div className="w-4 h-4 border-2 border-slate-500/70 border-t-emerald-400 rounded-full animate-spin"></div>
                                                    {"Geocoding..."}
                                                </>
                                            ) : (
                                                <>
                                                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z" />
                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M15 11a3 3 0 11-6 0 3 3 0 016 0z" />
                                                    </svg>
                                                    {"Geocode"}
                                                </>
                                            )}
                                        </button>
                                    )}
                                </div>

                                {whoisLoading ? (
                                    <div className="flex items-center gap-3 p-4 rounded-xl bg-slate-800/35 border border-slate-700/40">
                                        <div className="w-5 h-5 border-2 border-slate-500/70 border-t-blue-400 rounded-full animate-spin"></div>
                                        <span className="text-slate-300">{"Fetching WHOIS data..."}</span>
                                    </div>
                                ) : whoisData ? (
                                    <div className="space-y-4">
                                        <div className="p-4 rounded-2xl bg-slate-800/35 border border-slate-700/40 space-y-3">
                                            <h4 className="text-xs font-semibold text-slate-300 uppercase tracking-wider">{"Registration Info"}</h4>
                                            {whoisData.as_name && (
                                                <div className="flex flex-col sm:flex-row sm:items-start gap-1 sm:gap-3">
                                                    <span className="text-xs sm:text-sm text-slate-400 sm:w-28 flex-shrink-0 uppercase tracking-wide sm:normal-case sm:tracking-normal">
                                                        {"AS Name"}
                                                    </span>
                                                    <span className="text-sm text-slate-100/90 leading-snug break-words">{whoisData.as_name}</span>
                                                </div>
                                            )}
                                            {whoisData.descr.length > 0 && (
                                                <div className="flex flex-col sm:flex-row sm:items-start gap-1 sm:gap-3">
                                                    <span className="text-xs sm:text-sm text-slate-400 sm:w-28 flex-shrink-0 uppercase tracking-wide sm:normal-case sm:tracking-normal">
                                                        {"Description"}
                                                    </span>
                                                    <span className="text-sm text-slate-100/90 leading-snug break-words">
                                                        {whoisData.descr.join(" • ")}
                                                    </span>
                                                </div>
                                            )}
                                            {whoisData.country && (
                                                <div className="flex flex-col sm:flex-row sm:items-start gap-1 sm:gap-3">
                                                    <span className="text-xs sm:text-sm text-slate-400 sm:w-28 flex-shrink-0 uppercase tracking-wide sm:normal-case sm:tracking-normal">
                                                        {"Country"}
                                                    </span>
                                                    <span className="text-sm text-slate-100/90 leading-snug break-words">{whoisData.country}</span>
                                                </div>
                                            )}
                                        </div>

                                        {whoisData.organisation && (
                                            <div className="p-4 rounded-2xl bg-slate-800/35 border border-slate-700/40 space-y-3">
                                                <h4 className="text-xs font-semibold text-slate-300 uppercase tracking-wider">{"Organisation"}</h4>
                                                <div className="flex flex-col sm:flex-row sm:items-start gap-1 sm:gap-3">
                                                    <span className="text-xs sm:text-sm text-slate-400 sm:w-28 flex-shrink-0 uppercase tracking-wide sm:normal-case sm:tracking-normal">
                                                        {"Name"}
                                                    </span>
                                                    <span className="text-sm text-slate-100/90 leading-snug break-words">{whoisData.organisation.org_name}</span>
                                                </div>
                                                {whoisData.organisation.address.length > 0 && (
                                                    <div className="flex flex-col sm:flex-row sm:items-start gap-1 sm:gap-3">
                                                        <span className="text-xs sm:text-sm text-slate-400 sm:w-28 flex-shrink-0 uppercase tracking-wide sm:normal-case sm:tracking-normal">
                                                            {"Address"}
                                                        </span>
                                                        <span className="text-sm text-slate-100/90 leading-snug break-words">
                                                            {whoisData.organisation.address.join(", ")}
                                                        </span>
                                                    </div>
                                                )}
                                                {whoisData.organisation.email && (
                                                    <div className="flex flex-col sm:flex-row sm:items-start gap-1 sm:gap-3">
                                                        <span className="text-xs sm:text-sm text-slate-400 sm:w-28 flex-shrink-0 uppercase tracking-wide sm:normal-case sm:tracking-normal">
                                                            {"Email"}
                                                        </span>
                                                        <span className="text-sm text-slate-100/90 leading-snug break-words">{whoisData.organisation.email}</span>
                                                    </div>
                                                )}
                                            </div>
                                        )}

                                        {whoisData.contacts.length > 0 && (
                                            <div className="p-4 rounded-2xl bg-slate-800/35 border border-slate-700/40">
                                                <h4 className="text-xs font-semibold text-slate-300 uppercase tracking-wider mb-3">{"Technical Contacts"}</h4>
                                                <div className="space-y-2">
                                                    {whoisData.contacts.map((contact) => (
                                                        <div
                                                            key={contact.nic_hdl}
                                                            className="flex items-center justify-between gap-3 p-3 rounded-xl bg-slate-900/35 border border-slate-700/35"
                                                        >
                                                            <div className="flex items-center gap-3 min-w-0">
                                                                <div className="w-9 h-9 bg-slate-800/45 border border-slate-700/35 rounded-full flex items-center justify-center flex-shrink-0">
                                                                    <svg className="w-4 h-4 text-slate-300/70" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                                                                    </svg>
                                                                </div>
                                                                <div className="min-w-0">
                                                                    <p className="text-sm font-semibold text-slate-100 tracking-tight truncate">{contact.name}</p>
                                                                    <p className="text-xs text-slate-400 truncate">{contact.nic_hdl}</p>
                                                                </div>
                                                            </div>
                                                            {contact.email && (
                                                                <a
                                                                    href={`mailto:${contact.email}`}
                                                                    className="text-sm text-blue-300 hover:text-blue-200 font-semibold transition-colors flex-shrink-0 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-400/60 rounded-lg px-2 py-1"
                                                                >
                                                                    {contact.email}
                                                                </a>
                                                            )}
                                                        </div>
                                                    ))}
                                                </div>
                                            </div>
                                        )}
                                    </div>
                                ) : (
                                    <div className="p-4 rounded-xl bg-slate-800/30 border border-slate-700/35 text-center">
                                        <p className="text-slate-400">{"No WHOIS data available"}</p>
                                    </div>
                                )}
                            </div>

                            {geocodedAddresses.length > 0 && (
                                <div className="p-6 rounded-2xl bg-slate-900/40 border border-slate-800/60 backdrop-blur-sm shadow-[0_10px_40px_-25px_rgba(0,0,0,0.85)] transition-all duration-300 hover:border-slate-700/70 hover:shadow-[0_18px_60px_-35px_rgba(0,0,0,0.9)]">
                                    <div className="flex items-start gap-3 mb-5">
                                        <div className="p-2 bg-emerald-500/15 rounded-xl border border-emerald-500/20">
                                            <svg className="w-5 h-5 text-emerald-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z" />
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M15 11a3 3 0 11-6 0 3 3 0 016 0z" />
                                            </svg>
                                        </div>
                                        <div className="min-w-0">
                                            <h3 className="text-lg font-semibold text-white tracking-tight">{"Geocoding Results"}</h3>
                                            <p className="text-sm text-slate-400">{`${successful.length} of ${geocodedAddresses.length} addresses resolved`}</p>
                                        </div>
                                    </div>

                                    {successful.length > 0 && (
                                        <div className="space-y-3 mb-4">
                                            {successful.map((addr) => {
                                                const coord = addr.coordinate as Coord;
                                                const osmUrl = `https://www.openstreetmap.org/?mlat=${coord.lat.toFixed(6)}&mlon=${coord.lon.toFixed(6)}#map=15/${coord.lat.toFixed(6)}/${coord.lon.toFixed(6)}`;
                                                const gmapsUrl = `https://www.google.com/maps/search/?api=1&query=${coord.lat.toFixed(6)},${coord.lon.toFixed(6)}`;

                                                return (
                                                    <div key={`${addr.original_address}-${coord.lat}`} className="p-4 rounded-xl bg-emerald-500/10 border border-emerald-500/20">
                                                        <div className="flex items-start justify-between gap-4 mb-3">
                                                            <div className="min-w-0 flex-1">
                                                                <p className="text-sm text-slate-300 break-words mb-1">{addr.original_address}</p>
                                                                {addr.display_name && (
                                                                    <p className="text-xs text-slate-400 break-words">{`→ ${addr.display_name}`}</p>
                                                                )}
                                                            </div>
                                                            <div className="flex items-center gap-1 flex-shrink-0">
                                                                <a
                                                                    href={osmUrl}
                                                                    target="_blank"
                                                                    className="p-2 rounded-lg bg-slate-800/50 border border-slate-700/40 hover:bg-blue-500/20 hover:border-blue-500/30 transition-all focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-400/60"
                                                                    title="View on OpenStreetMap"
                                                                >
                                                                    <svg className="w-4 h-4 text-slate-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 20l-5.447-2.724A1 1 0 013 16.382V5.618a1 1 0 011.447-.894L9 7m0 13l6-3m-6 3V7m6 10l4.553 2.276A1 1 0 0021 18.382V7.618a1 1 0 00-.553-.894L15 4m0 13V4m0 0L9 7" />
                                                                    </svg>
                                                                </a>
                                                                <a
                                                                    href={gmapsUrl}
                                                                    target="_blank"
                                                                    className="p-2 rounded-lg bg-slate-800/50 border border-slate-700/40 hover:bg-red-500/20 hover:border-red-500/30 transition-all focus:outline-none focus-visible:ring-2 focus-visible:ring-red-400/60"
                                                                    title="View on Google Maps"
                                                                >
                                                                    <svg className="w-4 h-4 text-slate-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z" />
                                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M15 11a3 3 0 11-6 0 3 3 0 016 0z" />
                                                                    </svg>
                                                                </a>
                                                            </div>
                                                        </div>
                                                        <div className="flex items-center gap-2 text-xs">
                                                            <span className="px-2 py-1 bg-emerald-500/20 text-emerald-300 rounded-md font-mono">
                                                                {`${coord.lat.toFixed(5)}°`}
                                                            </span>
                                                            <span className="text-slate-500">{","}</span>
                                                            <span className="px-2 py-1 bg-emerald-500/20 text-emerald-300 rounded-md font-mono">
                                                                {`${coord.lon.toFixed(5)}°`}
                                                            </span>
                                                        </div>
                                                    </div>
                                                );
                                            })}
                                        </div>
                                    )}

                                    {failed.length > 0 && (
                                        <div className="space-y-2">
                                            <p className="text-xs text-slate-400 uppercase tracking-wider font-semibold">{"Failed to resolve"}</p>
                                            {failed.map((addr) => (
                                                <div key={addr.original_address} className="p-3 rounded-xl bg-red-500/10 border border-red-500/20">
                                                    <p className="text-sm text-slate-300 break-words mb-1">{addr.original_address}</p>
                                                    {addr.error && <p className="text-xs text-red-300/80">{`Error: ${addr.error}`}</p>}
                                                </div>
                                            ))}
                                        </div>
                                    )}
                                </div>
                            )}
                        </div>

                        <div className="space-y-6">
                            <div className="p-6 rounded-2xl bg-slate-900/40 border border-slate-800/60 backdrop-blur-sm shadow-[0_10px_40px_-25px_rgba(0,0,0,0.85)] transition-all duration-300 hover:border-slate-700/70 hover:shadow-[0_18px_60px_-35px_rgba(0,0,0,0.9)]">
                                <div className="flex items-center gap-3 mb-5">
                                    <div className="p-2 bg-amber-500/15 rounded-xl border border-amber-500/20">
                                        <svg className="w-5 h-5 text-amber-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M5 13l4 4L19 7" />
                                        </svg>
                                    </div>
                                    <div className="min-w-0">
                                        <h3 className="text-lg font-semibold text-white tracking-tight">{"Favorites"}</h3>
                                        <p className="text-sm text-slate-400">{"Lists and notes for this ASN"}</p>
                                    </div>
                                </div>

                                {userDataLoading ? (
                                    <p className="text-sm text-slate-400">{"Loading saved lists..."}</p>
                                ) : userData ? (
                                    <div className="space-y-4">
                                        <div className="space-y-2">
                                            {userData.lists.length ? (
                                                <div className="space-y-2">
                                                    {userData.lists.map((name) => (
                                                        <div
                                                            key={name}
                                                            className="flex items-center justify-between gap-2 rounded-lg border border-slate-700/50 bg-slate-950/40 px-2 py-1.5 text-xs text-slate-300"
                                                        >
                                                            <span className="truncate">{name}</span>
                                                            <button
                                                                onClick={() => removeList(name)}
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
                                                className="flex-1 px-3 py-2 bg-slate-950/70 border border-slate-700/50 rounded-lg text-xs text-slate-200 focus:outline-none focus:ring-2 focus:ring-amber-400/40 focus:border-amber-400/40"
                                                onChange={(e) => setListInput(e.target.value)}
                                            />
                                            <button
                                                onClick={addList}
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
                                                className="w-full px-3 py-2 bg-slate-950/70 border border-slate-700/50 rounded-lg text-xs text-slate-200 focus:outline-none focus:ring-2 focus:ring-amber-400/40 focus:border-amber-400/40"
                                                onChange={(e) => setCommentDraft(e.target.value)}
                                            />
                                            <button
                                                onClick={saveComment}
                                                className="mt-2 w-full px-3 py-2 text-xs font-semibold rounded-lg bg-slate-700/50 text-slate-200 border border-slate-600/40 hover:bg-slate-600/60 transition"
                                            >
                                                {"Save comment"}
                                            </button>
                                        </div>
                                    </div>
                                ) : (
                                    <p className="text-sm text-slate-400">{"No user data loaded"}</p>
                                )}
                            </div>

                            <div className="p-6 rounded-2xl bg-slate-900/40 border border-slate-800/60 backdrop-blur-sm shadow-[0_10px_40px_-25px_rgba(0,0,0,0.85)] transition-all duration-300 hover:border-slate-700/70 hover:shadow-[0_18px_60px_-35px_rgba(0,0,0,0.9)]">
                                <div className="flex items-center gap-3 mb-5">
                                    <div className="p-2 bg-blue-500/15 rounded-xl border border-blue-500/20">
                                        <svg className="w-5 h-5 text-blue-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                                        </svg>
                                    </div>
                                    <div className="min-w-0">
                                        <h3 className="text-lg font-semibold text-white tracking-tight">{"External Resources"}</h3>
                                        <p className="text-sm text-slate-400">{"Useful links for this ASN"}</p>
                                    </div>
                                </div>

                                <div className="space-y-2">
                                    {externalLinks.map((link) => (
                                        <a
                                            key={link.name}
                                            href={link.url}
                                            target="_blank"
                                            className={`flex items-center justify-between gap-3 p-3 rounded-xl text-white font-semibold tracking-tight bg-gradient-to-r ${link.gradient} shadow-sm hover:opacity-95 hover:translate-x-0.5 transition-all duration-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-white/30`}
                                        >
                                            <span className="truncate">{link.name}</span>
                                            <svg className="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M14 5l7 7m0 0l-7 7m7-7H3" />
                                            </svg>
                                        </a>
                                    ))}
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </main>
        </div>
    );
}
