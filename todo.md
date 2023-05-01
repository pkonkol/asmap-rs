# tasks
- asdbmaker execute external python script for asrank
- asdbmaker load asrank data into mongoDB
- asmap-fronend load asdbmaker data
- asmap-fronend serve empty map
- asmap-fronend show asns with cords from asdbmaker data 
        - asmap-fronend allow to expand details for selected ASn
- gather example whois output for all regional registries for as, org, person

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
### whois

### bgpview
detailed infor for prefixes, 
## db collections:
asns { }
organisations { }
persons {} # referenced in org or asn
prefixes {} # TODO for later, scarappable from bgpView API, 
### prefixes
Their data is often incomplete on bgpView API.
There also doesn't seem to be any geolocation data for prefixes
### asns
asrank provided ASns contain geolocation data (based on what?). Let's do a map
of that first.
### orgs
### persons
Are the persons in asn lookup the same as in org lookup? likely yes so 
### asnLinks (TODO later)
Would this be helpful? It's easily downloadable from asrank .py script.
It would allow a feature to see related asns for 

## TODO later
- scrape prefixes for given ASn (supported by bgpview, useful for scanning)

# asmap-frontend (so just asmap-rs)
create open street map based interactive map with located ASns or ORGs
## TODO now
  - make a map based on cords provided in asrank asns.jsonl
## TODO later
  - graphQL endpoint for querying orgs/asns by location
