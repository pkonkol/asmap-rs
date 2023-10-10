#!/usr/bin/bash

python -m venv venv
source venv/bin/activate
pip install graphqlclient
whois --help >/dev/null 2>&1 # check whether whois is installed
if [ ! -f "asrank-download.py" ]; then
    wget https://api.asrank.caida.org/dev/scripts/asrank-download.py
fi
if [ ! -d "inputs" ]; then
    mkdir inputs
fi
if [ ! -f "inputs/asns.jsonl" ]; then
    python3 asrank-download.py -v -a inputs/asns.jsonl -u https://api.asrank.caida.org/v2/graphql
fi
echo "all required files are in place"
