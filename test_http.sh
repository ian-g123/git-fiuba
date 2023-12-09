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
rm -rf repo_cliente_merge_conflicts2
rm -rf repo_merge_conflict
rm -rf repo_merge_conflict2
rm -rf repo_safe_merge
rm tmp-curl-response
unzip -qq repo_merge_conflict.zip -d repo_merge_conflict
unzip -qq repo_merge_conflict2.zip -d repo_merge_conflict2
unzip -qq repo_safe_merge.zip -d repo_safe_merge
../../../target/debug/git-server &
server_process=$!
sleep 1


# Get pull requests
echo "=========================="
echo "Get pull requests"
curl -s -o tmp-curl-response -L \
  -X GET \
  http://127.1.0.0:8080/repos/repo_safe_merge/pulls

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
response_content=$(cat tmp-curl-response)
expected_content='[]'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"

# Get pull request
echo "=========================="
echo "Get pull request"
curl -s -o tmp-curl-response -L \
  -X GET \
  http://127.1.0.0:8080/repos/repo_safe_merge/pulls/1

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
response_content=$(cat tmp-curl-response)
expected_content='Not Found'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"


# Creating Pull Request
echo "=========================="
echo "Creating Pull Request"
curl -s -o tmp-curl-response -L \
  -X POST \
  http://127.1.0.0:8080/repos/repo_safe_merge/pulls \
  -d '{"title":"Safe merge pull request","description":"My pull request description","sourceBranch":"rama","targetBranch":"master"}'

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
response_content=$(cat tmp-curl-response)
expected_content='{"id":1,"title":"Safe merge pull request","description":"My pull request description","sourceBranch":"rama","targetBranch":"master","state":"open","hasMergeConflicts":false,"merged":false}'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi

# Check if the pull request was created
echo "=========================="
echo "Check if the pull request was created"
if [ ! -f repo_safe_merge/server_files/pull_requests/1.json ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
if [ ! -f repo_safe_merge/server_files/LAST_PULL_REQUEST_ID ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
pull_request_content=$(cat repo_safe_merge/server_files/pull_requests/1.json)
expected_content='{"id":1,"title":"Safe merge pull request","description":"My pull request description","sourceBranch":"rama","targetBranch":"master","state":"open","merged":false}'
if [ "$pull_request_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $pull_request_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"
rm tmp-curl-response


# Get pull requests
echo "=========================="
echo "Get pull requests"
curl -s -o tmp-curl-response -L \
  -X GET \
  http://127.1.0.0:8080/repos/repo_safe_merge/pulls

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
response_content=$(cat tmp-curl-response)
expected_content='[{"id":1,"title":"Safe merge pull request","description":"My pull request description","sourceBranch":"rama","targetBranch":"master","state":"open","hasMergeConflicts":false,"merged":false}]'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"

# Get pull request
echo "=========================="
echo "Get pull request"
curl -s -o tmp-curl-response -L \
  -X GET \
  http://127.1.0.0:8080/repos/repo_safe_merge/pulls/1

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
response_content=$(cat tmp-curl-response)
expected_content='{"id":1,"title":"Safe merge pull request","description":"My pull request description","sourceBranch":"rama","targetBranch":"master","state":"open","hasMergeConflicts":false,"merged":false}'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"

# Creating Pull Request
echo "=========================="
echo "Creating Pull Request"
curl -s -o tmp-curl-response -L \
  -X POST \
  http://127.1.0.0:8080/repos/repo_safe_merge/pulls \
  -d '{"title":"Inverted Safe merge pull request","description":"My second pull request description","sourceBranch":"master","targetBranch":"rama"}'

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
response_content=$(cat tmp-curl-response)
expected_content='{"id":2,"title":"Inverted Safe merge pull request","description":"My second pull request description","sourceBranch":"master","targetBranch":"rama","state":"open","hasMergeConflicts":false,"merged":false}'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi

# Check if the pull request was created
echo "=========================="
echo "Check if the pull request was created"
if [ ! -f repo_safe_merge/server_files/pull_requests/2.json ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
pull_request_content=$(cat repo_safe_merge/server_files/pull_requests/2.json)
expected_content='{"id":2,"title":"Inverted Safe merge pull request","description":"My second pull request description","sourceBranch":"master","targetBranch":"rama","state":"open","hasMergeConflicts":false,"merged":false}'

# Get pull requests
echo "=========================="
echo "Get pull requests"
curl -s -o tmp-curl-response -L \
  -X GET \
  http://127.1.0.0:8080/repos/repo_safe_merge/pulls

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
response_content=$(cat tmp-curl-response)
expected_content='[{"id":1,"title":"Safe merge pull request","description":"My pull request description","sourceBranch":"rama","targetBranch":"master","state":"open","hasMergeConflicts":false,"merged":false},{"id":2,"title":"Inverted Safe merge pull request","description":"My second pull request description","sourceBranch":"master","targetBranch":"rama","state":"open","hasMergeConflicts":false,"merged":false}]'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"

# Get inexistant pull request
echo "=========================="
echo "Get inexistant pull request"
curl -s -o tmp-curl-response -L \
  -X GET \
  http://127.1.0.0:8080/repos/repo_safe_merge/pulls/3

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
response_content=$(cat tmp-curl-response)
expected_content='Not Found'

if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"

# Patch pull request closed
echo "=========================="
echo "Patch pull request closed"
curl -s -o tmp-curl-response -L \
  -X PATCH \
  http://127.1.0.0:8080/repos/repo_safe_merge/pulls/1 \
  -d '{"title":"Safe merge pull request modified", "state":"closed"}'

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi

response_content=$(cat tmp-curl-response)
expected_content='{"id":1,"title":"Safe merge pull request modified","description":"My pull request description","sourceBranch":"rama","targetBranch":"master","state":"closed","hasMergeConflicts":false,"merged":false}'

if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"


# Fail to merge pull request
echo "=========================="
echo "Fail to merge pull request"
curl -s -o tmp-curl-response -L \
  -X PUT \
  http://127.1.0.0:8080/repos/repo_safe_merge/pulls/1/merge

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
echo "✅"

response_content=$(cat tmp-curl-response)
expected_content='Pull request is closed'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"


# Patch pull request open
echo "=========================="
echo "Patch pull request open"
curl -s -o tmp-curl-response -L \
  -X PATCH \
  http://127.1.0.0:8080/repos/repo_safe_merge/pulls/1 \
  -d '{"title":"Safe merge pull request modified", "state":"open"}'

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi

response_content=$(cat tmp-curl-response)
expected_content='{"id":1,"title":"Safe merge pull request modified","description":"My pull request description","sourceBranch":"rama","targetBranch":"master","state":"open","hasMergeConflicts":false,"merged":false}'

if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"


# Success in merging pull request
echo "=========================="
echo "Success in merging pull request"
curl -s -o tmp-curl-response -L \
  -X PUT \
  http://127.1.0.0:8080/repos/repo_safe_merge/pulls/1/merge

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
echo "✅"

response_content=$(cat tmp-curl-response)
expected_content='{"id":1,"title":"Safe merge pull request modified","description":"My pull request description","sourceBranch":"rama","targetBranch":"master","state":"closed","hasMergeConflicts":false,"merged":true}'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"


# Fail to merge already merged pull request
echo "=========================="
echo "Fail to merge already merged pull request"
curl -s -o tmp-curl-response -L \
  -X PUT \
  http://127.1.0.0:8080/repos/repo_safe_merge/pulls/1/merge

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
echo "✅"

response_content=$(cat tmp-curl-response)
expected_content='Pull request is already merged'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"


# Check if the pull request was merged
echo "=========================="
echo "Check if the pull request was merged"
cd repo_safe_merge
last_commit_hash=$(cat refs/heads/master)
last_commit=$(git cat-file -p $last_commit_hash)
# Cut the first 5 lines
last_commit=$(echo "$last_commit" | tail -n +7)
expected_content='Merge pull request #1 from rama

Safe merge pull request modified
My pull request description'
if [ "$last_commit" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $last_commit"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"
cd -


###########################################################
# Pull Request with merge conflict and resolve through API
echo "############################################################"
echo "# Pull Request with merge conflict and resolve through API #"
echo "############################################################"
echo "=========================="
echo "Creating Pull Request with merge conflict"
curl -s -o tmp-curl-response -L \
  -X POST \
  http://127.1.0.0:8080/repos/repo_merge_conflict2/pulls \
  -d '{"title":"Merge conflict 2 pull request","description":"My pull request description","sourceBranch":"rama","targetBranch":"master"}'

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi

response_content=$(cat tmp-curl-response)
expected_content='{"id":1,"title":"Merge conflict 2 pull request","description":"My pull request description","sourceBranch":"rama","targetBranch":"master","state":"open","hasMergeConflicts":true,"merged":false}'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"

# Fail to merge pull request cause there are merge conflicts
echo "=========================="
echo "Fail to merge pull request cause there are merge conflicts"
curl -s -o tmp-curl-response -L \
  -X PUT \
  http://127.1.0.0:8080/repos/repo_merge_conflict2/pulls/1/merge

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
echo "✅"

response_content=$(cat tmp-curl-response)
expected_content='Merge conflict! Error: There are conflicts in the working directory'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"

# Patch pull request open
echo "=========================="
echo "Patch pull request open"
curl -s -o tmp-curl-response -L \
  -X PATCH \
  http://127.1.0.0:8080/repos/repo_merge_conflict2/pulls/1 \
  -d '{"title":"Conflict merge pull request modified", "state":"open"}'

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi

response_content=$(cat tmp-curl-response)
expected_content='{"id":1,"title":"Conflict merge pull request modified","description":"My pull request description","sourceBranch":"rama","targetBranch":"master","state":"open","hasMergeConflicts":true,"merged":false}'

if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"


# Resolve merge conflicts by the client
cd ..
cd client_files
rm -rf repo_cliente_merge_conflicts2
unzip -qq repo_cliente_merge_conflicts2.zip -d repo_cliente_merge_conflicts2

cd repo_cliente_merge_conflicts2
../../../../target/debug/git checkout master
echo "contenido compartido" > fu
# touch woo
echo "contenido woo" > woo
../../../../target/debug/git add .
../../../../target/debug/git commit -m "Modifico_master"
../../../../target/debug/git push

../../../../target/debug/git checkout rama
echo "contenido compartido" > fu
# touch foo
echo "contenido foo" > foo
../../../../target/debug/git add .
../../../../target/debug/git commit -m "Modifico_rama"
../../../../target/debug/git push


cd ..
rm -rf repo_cliente_merge_conflicts2
cd ..
cd server_files


# Success in merging pull request
echo "=========================="
echo "Success in merging pull request"
curl -s -o tmp-curl-response -L \
  -X PUT \
  http://127.1.0.0:8080/repos/repo_merge_conflict2/pulls/1/merge

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
echo "✅"

response_content=$(cat tmp-curl-response)
expected_content='{"id":1,"title":"Conflict merge pull request modified","description":"My pull request description","sourceBranch":"rama","targetBranch":"master","state":"closed","hasMergeConflicts":false,"merged":true}'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"



###########################################################
# Pull Request with merge conflict and resolved through push
echo "#############################################################"
echo "# Pull Request with merge conflict and resolved through push #"
echo "#############################################################"
echo "=========================="
echo "Creating Pull Request with merge conflict"
curl -s -o tmp-curl-response -L \
  -X POST \
  http://127.1.0.0:8080/repos/repo_merge_conflict/pulls \
  -d '{"title":"Merge conflict resolved through push","description":"My pull request description","sourceBranch":"rama","targetBranch":"master"}'

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi

response_content=$(cat tmp-curl-response)
expected_content='{"id":1,"title":"Merge conflict resolved through push","description":"My pull request description","sourceBranch":"rama","targetBranch":"master","state":"open","hasMergeConflicts":true,"merged":false}'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"

# Fail to merge pull request cause there are merge conflicts
echo "=========================="
echo "Fail to merge pull request cause there are merge conflicts"
curl -s -o tmp-curl-response -L \
  -X PUT \
  http://127.1.0.0:8080/repos/repo_merge_conflict/pulls/1/merge

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
echo "✅"

response_content=$(cat tmp-curl-response)
expected_content='Merge conflict! Error: There are conflicts in the working directory'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"

# Resolve merge conflicts by the client
cd ../client_files
rm -rf repo_merge_conflict
../../../target/debug/git clone git://127.1.0.0:9418/repo_merge_conflict
cd repo_merge_conflict

sleep 1

echo "Linea 1
Linea 2
Linea 3" > file
../../../../target/debug/git add .
../../../../target/debug/git commit -m "ResoluciónConflictos"
../../../../target/debug/git push

sleep 1

# Checking if pull request conflicts where resolved
echo "=========================="
echo "Checking if pull request conflicts where resolved"
curl -s -o tmp-curl-response -L \
  -X GET \
  http://127.1.0.0:8080/repos/repo_merge_conflict/pulls/1

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
echo "✅"

response_content=$(cat tmp-curl-response)
expected_content='{"id":1,"title":"Merge conflict resolved through push","description":"My pull request description","sourceBranch":"rama","targetBranch":"master","state":"open","hasMergeConflicts":false,"merged":false}'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"

../../../../target/debug/git merge rama
../../../../target/debug/git push

sleep 1

# Checking if pull request was merged
echo "=========================="
echo "Checking if pull request was merged"
curl -s -o tmp-curl-response -L \
  -X GET \
  http://127.1.0.0:8080/repos/repo_merge_conflict/pulls/1

if [ ! -f tmp-curl-response ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
echo "✅"

response_content=$(cat tmp-curl-response)
expected_content='{"id":1,"title":"Merge conflict resolved through push","description":"My pull request description","sourceBranch":"rama","targetBranch":"master","state":"closed","hasMergeConflicts":false,"merged":true}'
if [ "$response_content" != "$expected_content" ]; then
    echo "❌ Failed. Actual content is not equal to expected content:"
    echo "Actual:   $response_content"
    echo "Expected: $expected_content"
    kill $server_process
    exit 1
fi
echo "✅"

rm tmp-curl-response
cd ..
rm -rf repo_merge_conflict
cd ../server_files

# Remove all the files that were created
rm -rf repo_merge_conflict2
rm -rf repo_merge_conflict
rm -rf repo_safe_merge
rm tmp-curl-response
rm http-server-logs.log
rm tcp-server-logs.log
kill $server_process

