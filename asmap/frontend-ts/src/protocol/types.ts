export type AsFiltersHasOrg = "Yes" | "No" | "Both";

export interface Coord {
    lat: number;
    lon: number;
}

export interface Bound {
    north_east: Coord;
    south_west: Coord;
}

export interface AsFilters {
    country: string | null;
    exclude_country: boolean;
    bounds: Bound | null;
    addresses: [number, number] | null;
    rank: [number, number] | null;
    has_org: AsFiltersHasOrg;
    category: string[];
}

export interface AsForFrontend {
    asn: number;
    rank: number;
    name: string;
    country_code: string;
    organization: string | null;
    prefixes: number;
    addresses: number;
    coordinates: Coord;
}

export interface AsrankDegree {
    provider: number;
    peer: number;
    customer: number;
    total: number;
    transit: number;
    sibling: number;
}

export interface AsrankAsn {
    rank: number;
    organization: string | null;
    country_iso: string;
    country_name: string;
    coordinates: Coord;
    degree: AsrankDegree;
    prefixes: number;
    addresses: number;
    name: string;
}

export type InternetRegistry =
    | "RIPE"
    | "ARIN"
    | "APNIC"
    | "AFRINIC"
    | "LACNIC"
    | "EMPTY"
    | { LOCAL: string };

export interface IPNetDBIX {
    exchange: string;
    ipv4: number[] | null;
    ipv6: number[] | null;
    name: string | null;
    speed: number;
}

export interface IPNetDBPrefixDetails {
    allocation: string | null;
    allocation_cc: string | null;
    allocation_registry: InternetRegistry | null;
    prefix_entity: string;
    prefix_name: string;
    prefix_origins: number[];
    prefix_registry: string;
}

export interface IPNetDBPrefix {
    range: string;
    details: IPNetDBPrefixDetails | null;
}

export interface IPNetDBAsn {
    cc: string;
    entity: string;
    in_use: boolean;
    ipv4_prefixes: IPNetDBPrefix[];
    ipv6_prefixes: IPNetDBPrefix[];
    name: string | null;
    peers: number[];
    private: boolean;
    registry: InternetRegistry;
    status: string | null;
    ix: IPNetDBIX[];
}

export interface StanfordASdbCategory {
    layer1: string;
    layer2: string;
}

export interface WhoIsOrg {
    org_id: string;
    org_name: string;
    org_type: string | null;
    address: string[];
    country: string | null;
    phone: string | null;
    email: string | null;
}

export interface WhoIsPerson {
    nic_hdl: string;
    name: string;
    address: string[];
    phone: string | null;
    email: string | null;
}

export interface WhoIsAsn {
    as_name: string | null;
    descr: string[];
    org_id: string | null;
    admin_c: string[];
    tech_c: string[];
    abuse_c: string | null;
    country: string | null;
    organisation: WhoIsOrg | null;
    contacts: WhoIsPerson[];
    fetched_at: string | null;
}

export interface As {
    asn: number;
    asrank_data: AsrankAsn | null;
    ipnetdb_data: IPNetDBAsn | null;
    whois_data: WhoIsAsn | null;
    stanford_asdb: StanfordASdbCategory[];
}

export type WSRequest =
    | { FilteredAS: AsFilters }
    | { AsDetails: number }
    | { FetchWhois: number }
    | { GetWhois: number };

export type WSResponse =
    | { FilteredAS: [AsFilters, AsForFrontend[]] }
    | { AsDetails: As }
    | { WhoisData: WhoIsAsn | null }
    | { Error: string };
