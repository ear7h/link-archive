set -e

if [ $# -ne 1 ]; then
	echo "usage: ./new-migration.sh migration-name" >&2
	exit 1
fi

file_name=$(date +"%Y-%m-%d-$1.sql")
dir=$(git rev-parse --show-toplevel)/sql/migrations

if [ -e $dir/$file_name ]; then
	echo "$file_name already exists"
	exit 1
fi


set +e
read -r -d '' data<<EOF
PRAGMA foreign_keys = ON;

BEGIN EXCLUSIVE;

INSERT INTO migrations (name) VALUES ('$file_name');

END;
EOF
set -e

echo "$data" > $dir/$file_name

