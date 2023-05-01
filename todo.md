# tasks
- create mongoDB models
- separate asdb and asdbmaker
- asdbmaker load asrank data into mongoDB
- asmap-fronend serve empty map
- asmap-fronend load asdbmaker data
- asmap-fronend show asns with cords from asdbmaker data 
- asmap-fronend allow to expand details for selected ASn
## backlog
- gather example whois output for all regional registries for as, org, person
- asdbmaker download asrank using graphql
- find a better way to download whois data than executing `whois` for each prefix
- compare available georesolution APIs 


# idea
1 lib crate to parse publicly available APIs and put them into a DB
    - get all ASNs
    - for each ASn get whois data and scrape addresses for organization
1 lib crate to resolve adresses to geolocations
    - take all ASns DB and 
1 lib crate to create UI and interactive map with AS filtering
    - rust based UI so probably yew?

binary top crate to take crate 1, then resolve its geoloc using crate two and to launch UI from crate 3 if needed
    - use 2 binaries
      - to scrape and geolocate data
      - to start map server
        - at the end i should add graphQL endpoint to the server for API requests? return nearest based on geo cords
        - allow to show on the map:
          - ASNs by asrank geoloc
          - Organisations by georesolved address
          - Persons by georesolved address

# crate integration
how to transport data between crates? Same DB, option of import and export. 
    mongodump (bson) or mongoexport (json)? TODO decide later
Should all crates use common db instance? Yes
    Binary responsible for scraping and geolocating would both write to the DB. All will write to orgs/persons colletions
    but that shouldn't be a problem.
    Asdbmaker will write to the DB itself, georesolve's results will be written to the DB by the binary.
    We could georesolve by organisation and related persons. I assume each of these can have only one
    `address` value split into multiple lines so for each of these will be on `georesolved_adress` field
    which will be null by default.

Both binaries will allow import and export of the database.
How will binaries integrate with libs?
    asdbmaker will provide API to:
        create template database
        download and insert asrank data
        download details for a given ASn/ORG (from whois and bgpview)
            prefix details
    georesolve will provide API to:
        resolve coordinates for given address string

asmap-frontend will be the main binary
    This doesn't seem like a good match for a lib crate. So this would be a top level server binary.
    It's dependency will be a database with all required data that should be printed. Minimally this
    DB may be just organisations list with georesolved coords but that wouldn't be helpful, so I'll
    have to consider more complete data requirements for it.

# georesolve
- take in string or set of strings and return coordinates

# asdb
models and objects for the DB which can be used by asdbmaker but also by `asmap-rs`
frontend to read the data and by `scraper` to update `georesolved` data.

# asdbmaker
## functionality
- ASn no. & names ~~generate DB overview~~ (just top lvl ASn data? what about orgs)
- Orgs no.? & names 
- might have to resolve whois address data and geolocation together
    Scraping all whois data, and only then resolving adresses will likely be too inefficient
- generate DB details (which?)

## sources
### asrank
downloaded through the python script, provides a basic list of orgs and ansn
orgName provided isn't understood by whois unfortunately
How to translate this orgName into the one used by whois or at first just by RIPE?

### whois
What is the limit of whois requests before a ban? Prob different servers like ripe or radb have different
limits.
This may become the main one, right now I'm using the `jwhois` tool as it has all the servers updated.
How to find prefixes belonging to a given AS?
Prefixes for an AS may have different organization than the AS itself so they need to be mapped additionaly.
Example is AS6830 which has prefixes for UPC and many other ISPs

Can I batch whois entries?

I can get prefixes like this `whois -h whois.radb.net '!gas714'`, doesn't work for `ripe.net`.

#### persons
some have `remarks` with PGP key mention like `PGP: PGPKEY-CABC6580`
`nic-hdl` field seems to be the id from `tech-c` or `admin-c`
`address` is sometimes a multiline string and sometimes multiples `address` fields each being
    a single line


### ipnetdb.com
Free, contains all prefixes with as, org, registry data.
Managed to dump it to json, now would have to insert it into mongo.
I could merge this data with asrank provided but it doesn't give me any special benefits
beside containing the rank.
Still would have to translate the full name into whois understandable value. `allocation registry`
may be helpful here.
Ok, whatever xD I can just whois `prefix`. This gives me matching `org-name`, shorter whoisable
`organisation` field, `address` for given organisation, `org-type` which TODO check, 
`admin-c` and `tech-c` fields with `persons` and their `adresses` related to the org.

The problem is that there are like 1_100_000 prefixes so requesting each of them individually and
then few persons data would likely take a looot of time.
With no bans, assuming 1s per request doing just the prefixes without persons will takie 305 hours.
Batching or just scraping it partially, like for `PL` only.

### alternative
ipinfo.io
    Has anb API but for 150k requests it's 100$/month
bgp.he.net
bgpview
    Initially was meant to be the source for prefixes data but ipnetdb is better

## db collections:
asns { }
organisations { }
persons {}
prefixes {} 
### prefixes
The main object for my georesolve necessities, orgs and persons are most accurately ascribed to
prefixes. I have them all from ipnetdb. Just need the addresses.
### asns
asrank provided ASns contain geolocation data (based on what?). Let's do a map
of that first.
### orgs
Taken from the prefixes data.
### persons
Must bee looked up inidividually in whois but sometimes ripe returns them in single req.
TODO verify.

### asnLinks (TODO later)
Would this be helpful? It's easily downloadable from asrank .py script.
It would allow a feature to see related asns.
Maybe this can be skipped and this data retrieved as complex queries for prefixes and asns.

# asmap-frontend (so just asmap-rs)
create open street map based interactive map with located ASns or ORGs
## TODO now
  - make a map based on cords provided in asrank asns.jsonl
## TODO later
  - graphQL endpoint for querying orgs/asns by location

# Journal
## 01.05.23
For the first stage I just need data from the asrank asns.jsonl inserted into mongo. Then I will
use the provided geolocation for the yew frontend. 
Later I'll go and update DB with data from ipnetd with prefixes and organizations takend from
ipnetdb. I can make organisations collection based on what's in the prefix data. To geolocate
this data I'll have to make continous `whois` requests for each organization, then for each org
multiple requests for each `admin-c` and `tech-c`.

BGPView API seems to be unnecessary. Asrank orgs seem to be useless too. Asrank `asnlinks` may
have some use later but this data might be already in `ipnetdb`

