# Check if port 8080 is available
echo "=========================="
echo "Check if port 8080 is available"
if lsof -Pi :8080 -sTCP:LISTEN -t >/dev/null ; then
    echo "❌ Failed. Port 8080 is not available"
    exit 1
fi

# Build the server
echo "=========================="
echo "Building the server"
RUSTFLAGS="-Awarnings" cargo build -q -p git-server
 
# Run the server
echo "=========================="
echo "Run the server"
cd integration_tests/test_http_data/server_files
rm -rf repo_merge_conflict
rm -rf repo_safe_merge
unzip repo_merge_conflict.zip
unzip repo_safe_merge.zip
../../../target/debug/git-server &
daemon_process=$!
sleep 1

# Creating Pull Request
echo "=========================="
echo "Creating Pull Request"
response=$(curl -L \
  -X POST \
  http://127.1.0.0:8080/repos/repo_safe_merge/pulls \
  -d '{"title":"Safe merge pull request","body":"My pull request description","head":"rama","base":"master"}')

echo $response

# Check if the pull request was created
echo "=========================="
echo "Check if the pull request was created"
if [ ! -f repo_safe_merge/pull_requests/1.json ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
if [ ! -f repo_safe_merge/pull_requests/LAST_PULL_REQUEST_ID ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
pull_request_content=$(cat repo_safe_merge/pull_requests/1.json)
if [ "$pull_request_content" != "{}" ]; then
    echo "❌ Failed. Branches are not equal"
    kill $server_process
    exit 1
fi
last_pull_request_id=$(cat repo_safe_merge/pull_requests/LAST_PULL_REQUEST_ID)
if [ "$last_pull_request_id" != "{}" ]; then
    echo "❌ Failed. Branches are not equal"
    kill $server_process
    exit 1
fi