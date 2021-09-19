set -e

if [ $# -ne 1 ]; then
	echo "usage: ./run-migrations.sh database" >&2
	exit 1
fi

latest=$(sqlite3 $1 "SELECT name FROM migrations ORDER BY name DESC LIMIT 1" || true)


dir=$(git rev-parse --show-toplevel)/sql/migrations

for file in $(ls $dir | sort); do
	if [ $file ">" "$latest" ]; then
		echo "running $file"
		cat $dir/$file | sqlite3 -bail $1
	fi
done
