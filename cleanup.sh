#!/usr/bin/env bash
# remove database, files generated, files downloaded, cargo artifacts, node_modules, and dist
# flags: --all --generated --downloaded --database --cargo --frontend

clean_cargo=false
clean_frontend=false
clean_database=false
clean_downloaded=false

# Parse arguments manually (getopt behaves differently on macOS vs Linux)
while [[ $# -gt 0 ]]; do
    case "$1" in 
        -a|--all)
            clean_cargo=true
            clean_frontend=true
            clean_database=true
            clean_downloaded=true
            shift ;;
        -g|--generated)
            clean_cargo=true
            shift ;;
        -d|--downloaded)
            clean_downloaded=true
            shift ;;
        -b|--database)
            clean_database=true
            shift ;;
        -c|--cargo)
            clean_cargo=true
            shift ;;
        -f|--frontend)
            clean_frontend=true
            shift ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [-a|--all] [-g|--generated] [-d|--downloaded] [-b|--database] [-c|--cargo] [-f|--frontend]"
            exit 1 ;;
    esac
done

if [ "$clean_downloaded" = true ]; then
    echo "Cleaning downloaded"
    # Safety check: verify we're in the correct repository
    if [ ! -f "Cargo.toml" ] || [ ! -d "asmap/frontend-ts" ]; then
        echo "Error: Not in asmap-gis repository root. Aborting inputs/ deletion."
        exit 1
    fi

    inputs_path=$(cd . && pwd)/inputs
    if [ -d "$inputs_path" ]; then
        read -p "Delete downloaded inputs at '$inputs_path'? [y/N] " confirmation
        case "$confirmation" in
            y|Y|yes|YES)
                rm -rf "$inputs_path"
                echo "✓ Deleted $inputs_path"
                ;;
            *)
                echo "✗ Deletion cancelled"
                ;;
        esac
    fi
fi

if [ "$clean_database" = true ]; then
    echo "Cleaning database..."
    docker-compose down -v
fi

if [ "$clean_cargo" = true ]; then
    echo "Cleaning cargo artifacts..."
    cargo clean
    find . -type d -name "target" -exec rm -rf {} + 2>/dev/null
fi

if [ "$clean_frontend" = true ]; then
    echo "Cleaning frontend build artifacts..."
    rm -rf asmap/frontend-ts/dist
    rm -rf asmap/frontend-ts/node_modules
    echo "Removed dist/ and node_modules/"
fi

echo "done"