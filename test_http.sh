# Check if port 8080 is available
echo "=========================="
echo "Checking if port 8080 is available"
if lsof -Pi :8080 -sTCP:LISTEN -t >/dev/null ; then
    echo "❌ Failed. Port 8080 is not available"
    exit 1
fi
echo "✅"

# Build the server
echo "=========================="
echo "Building the server"
cargo build -q -p git-server
# RUSTFLAGS="-Awarnings" cargo build -q -p git-server
 
# Run the server
echo "=========================="
echo "Run the server"
cd integration_tests/test_http_data/server_files
rm -rf repo_merge_conflict
rm -rf repo_safe_merge
unzip -qq repo_merge_conflict.zip -d repo_merge_conflict
unzip -qq repo_safe_merge.zip -d repo_safe_merge
../../../target/debug/git-server &
server_process=$!
sleep 1

# Creating Pull Request
echo "=========================="
echo "Creating Pull Request"
response=$(curl -L \
  -X POST \
  http://127.1.0.0:8080/repos/repo_safe_merge/pulls \
  -d '{"title":"Safe merge pull request","description":"My pull request description","sourceBranch":"rama","targetBranch":"master"}')

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
expected_content='{"id":1,"title":"Safe merge pull request","description":"My pull request description","sourceBranch":"rama","targetBranch":"master","status":"open"}'
if [ "$pull_request_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $pull_request_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"

rm -rf repo_merge_conflict
rm -rf repo_safe_merge
rm http-server-logs.log
rm tcp-server-logs.log
kill $server_process