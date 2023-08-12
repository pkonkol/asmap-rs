#!/usr/bin/bash

python -m venv venv
source venv/bin/activate
pip install graphqlclient
if [ ! -f "asrank-download.py" ]; then
    wget https://api.asrank.caida.org/dev/scripts/asrank-download.py
fi
if [ ! -d "asdbmaker/inputs" ]; then
    mkdir asdbmaker/inputs
fi
if [ ! -f "asdbmaker/inputs/asns.jsonl" ]; then
    python3 asrank-download.py -v -a asdbmaker/inputs/asns.jsonl -u https://api.asrank.caida.org/v2/graphql
fi
echo "all required files are in place"
