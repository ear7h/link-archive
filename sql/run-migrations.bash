set -e

if [ $# -ne 1 ]; then
	echo "usage: ./run-migrations.sh migration-name" >&2
	exit 1
fi

latest=$(sqlite3 $1 "SELECT name FROM migrations ORDER BY name DESC LIMIT 1")

for file in $(ls migrations | sort); do
	if [ $file ">" $latest ]; then
		echo "running $file"
		cat migrations/$file | sqlite3 -bail $1
	fi
done
