#!/usr/bin/bash
# remove database, files generated, files downloaded
# flags: --all --generated --downloaded --database

# remove database with volumes
OPTS=`getopt -o agdb -l all,generated,downloaded,database -- "$@"`
if [[ $? -ne 0 ]]; then
    exit 1;
fi
eval set -- "$OPTS"

while true; do
    case "$1" in 
        -a|--all)
            docker-compose down -v; shift ;;
        -g|--generated)
            shift ;;
        -d|--downloaded)
            shift ;;
        -b|--database)
            docker-compose down -v; shift ;;
        --) shift; break ;;
        *) eho "Error"; exit 1 ;;
    esac
done

echo "done"