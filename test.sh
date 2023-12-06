echo "Generic tests"
RUSTFLAGS="-Awarnings" cargo test -q -p git -p git-lib -p git-server

# Run log_integration.rs test
echo "=========================="
echo "Log Integration test"
unzip -qq git/tests/data/commands/log.zip -d git/tests/data/commands/
RUSTFLAGS="-Awarnings" cargo test -q -p git -p git-lib -p git-server --test log_integration -- --ignored

# Check if port 9418 is available
echo "=========================="
echo "Check if port 9418 is available"
if lsof -Pi :9418 -sTCP:LISTEN -t >/dev/null ; then
    echo "❌ Failed. Port 9418 is not available"
    exit 1
fi

# Run clone_integration.rs test
echo "=========================="
echo "Clone Integration test"
mkdir -p git/tests/data/commands/clone/test1/server-files
cd git/tests/data/commands/clone/test1/server-files
git daemon --log-destination=none --reuseaddr --enable=receive-pack --informative-errors --base-path=. . & > "daemon.log"
daemon_process=$!
cd -
sleep 1
RUSTFLAGS="-Awarnings" cargo test -q -p git -p git-lib -p git-server --test clone_integration -- --ignored
kill $daemon_process

# Run push_integration.rs test
echo "=========================="
echo "Push Integration test"
unzip -qq git/tests/data/commands/push/test1/server-files/repo_backup_push.zip -d git/tests/data/commands/push/test1/server-files/
cd git/tests/data/commands/push/test1/server-files
git daemon --log-destination=none --reuseaddr --enable=receive-pack --informative-errors --base-path=. . & > "daemon.log"
daemon_process=$!
cd -
sleep 1
RUSTFLAGS="-Awarnings" cargo test -q -p git -p git-lib -p git-server --test push_integration -- --ignored
kill $daemon_process

# Run packfile_database_integration.rs test
echo "=========================="
echo "Packfile Integration test"
rm -rf git-lib/tests/data/packfile/simple_packfile
rm -rf git-lib/tests/data/packfile/simple_delta
rm -rf git-lib/tests/data/packfile/big_delta
rm -rf git-lib/tests/data/packfile/really_big_delta
unzip -qq git-lib/tests/data/packfile/simple_packfile.zip -d git-lib/tests/data/packfile/
unzip -qq git-lib/tests/data/packfile/simple_delta.zip -d git-lib/tests/data/packfile/
unzip -qq git-lib/tests/data/packfile/big_delta.zip -d git-lib/tests/data/packfile/
unzip -qq git-lib/tests/data/packfile/really_big_delta.zip -d git-lib/tests/data/packfile/
RUSTFLAGS="-Awarnings" cargo test -q -p git -p git-lib -p git-server --test packfile_database_integration -- --ignored

# Run delta_objects_over_network_client.rs test
echo "=========================="
echo "Delta over network test"
unzip -qq git/tests/data/commands/labdeltaclient/server_files/repo_with_two_commits.zip -d git/tests/data/commands/labdeltaclient/server_files/repo_with_two_commits
unzip -qq git/tests/data/commands/labdeltaclient/user2_to_recieve_delta.zip -d git/tests/data/commands/labdeltaclient/user2_to_recieve_delta
cd git/tests/data/commands/labdeltaclient/server_files
git daemon --log-destination=none --reuseaddr --enable=receive-pack --informative-errors --base-path=. . & > "daemon.log"
daemon_process=$!
cd -
sleep 1
RUSTFLAGS="-Awarnings" cargo test -q -p git -p git-lib -p git-server --test delta_objects_over_network_client -- --ignored
kill $daemon_process

# Run server_test
echo "=========================="
echo "Server test"
rm -rf git-server/tests/data/test1/server_files/repo
rm -rf git-server/tests/data/test1/server_files/repo_backup
unzip -qq git-server/tests/data/test1/server_files/repo_backup.zip -d git-server/tests/data/test1/server_files/
cd git-server/tests/data/test1/server_files
../../../../../target/debug/git-server &
daemon_process=$!
cd -
sleep 1

success=0
echo "First try"
for i in {1..10}
do
    echo "Attempt $i"
    echo "Server test attempt $i" > server-test-stderr.txt
    stdout=$(RUSTFLAGS="-Awarnings" cargo test -q -p git -p git-lib -p git-server --test server_test -- --ignored 2>server-test-stderr.txt)
    exit_status=$?
    if [ $exit_status -eq 0 ]; then
        success=1
        kill $daemon_process
        break
    fi
        echo "❌ Failed. Trying again"
        sleep 0.5
done

if [ $success -eq 0 ]; then
    echo "Failed 10 times. Exiting"
    cat server-test-stderr.txt
    kill $daemon_process
    echo "Try running"
    echo "RUSTFLAGS="-Awarnings" cargo test -q -p git -p git-lib -p git-server --test server_test -- --ignored"
    exit 1
fi
echo "✅ Passed"
rm server-test-stderr.txt


# Run delta_objects_server
echo "=========================="
echo "Server test with deltas"
rm -rf git-server/tests/data/test_delta_objects/server_files/repo
rm -rf git-server/tests/data/test_delta_objects/server_files/repo_backup
unzip -qq git-server/tests/data/test_delta_objects/server_files/repo_backup.zip -d git-server/tests/data/test_delta_objects/server_files/repo_backup
unzip -qq git-server/tests/data/test_delta_objects/user1_to_send_delta.zip -d git-server/tests/data/test_delta_objects/user1_to_send_delta
cd git-server/tests/data/test_delta_objects/server_files
../../../../../target/debug/git-server &
daemon_process=$!
cd -
sleep 1

success=0
echo "First try"
for i in {1..10}
do
    echo "Attempt $i"
    echo "Server test attempt $i" > server-test-stderr.txt
    stdout=$(RUSTFLAGS="-Awarnings" cargo test -q -p git -p git-lib -p git-server --test delta_objects_server -- --ignored 2>server-test-stderr.txt)
    exit_status=$?
    if [ $exit_status -eq 0 ]; then
        success=1
        kill $daemon_process
        break
    fi
        echo "❌ Failed. Trying again"
        sleep 0.5
done

if [ $success -eq 0 ]; then
    echo "Failed 10 times. Exiting"
    cat server-test-stderr.txt
    kill $daemon_process
    echo "Try running"
    echo "RUSTFLAGS="-Awarnings" cargo test -q -p git -p git-lib -p git-server --test delta_objects_server -- --ignored"
    exit 1
fi
echo "✅ Passed"
rm server-test-stderr.txt

# Run Integration Test
# We tests a custom git client against a custom git server so we use target/debug/git binnary
echo "=========================="
echo "Server-Client Integration test"
rm -rf server_client_integration_test
mkdir server_client_integration_test
mkdir server_client_integration_test/server
mkdir server_client_integration_test/client
cd server_client_integration_test/server
../../target/debug/git init --bare repo
../../target/debug/git-server & > "server_terminal.log"
server_process=$!
cd -
sleep 1
cd server_client_integration_test/client
../../target/debug/git clone git://127.1.0.0:9418/repo user1
cd user1
echo "Contenido Incial" > file
../../../target/debug/git add file
../../../target/debug/git commit -m InitialCommit
../../../target/debug/git push
cd -

sleep 1

master_branch_user_1=$(cat user1/.git/refs/heads/master)
../../target/debug/git clone git://127.1.0.0:9418/repo user2
if [ ! -f user2/.git/refs/heads/master ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
master_branch_user_2=$(cat user2/.git/refs/heads/master)
if [ "$master_branch_user_1" != "$master_branch_user_2" ]; then
    echo "❌ Failed. Branches are not equal"
    kill $server_process
    exit 1
fi

echo "✅ Primer push"

sleep 1

cd user1
../../../target/debug/git branch rama
echo "Contenido Master" > file1
../../../target/debug/git add file1
../../../target/debug/git commit -m MasterCommit
../../../target/debug/git checkout rama
echo "Contenido Rama" > file2
../../../target/debug/git add file2
../../../target/debug/git commit -m RamaCommit
echo "Contenido Rama" > file3
../../../target/debug/git add file3
../../../target/debug/git commit -m RamaCommit2
../../../target/debug/git push
../../../target/debug/git checkout master
../../../target/debug/git push

cd -

echo "✅ Push con varias ramas"

sleep 1

cd user2
../../../target/debug/git pull
../../../target/debug/git checkout rama
../../../target/debug/git pull
../../../target/debug/git checkout master
cd -
if [ ! -f user2/.git/refs/heads/master ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
if [ ! -f user2/.git/refs/heads/rama ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
master_branch_user_1=$(cat user1/.git/refs/heads/master)
master_branch_user_2=$(cat user2/.git/refs/heads/master)
if [ "$master_branch_user_1" != "$master_branch_user_2" ]; then
    echo "❌ Failed. Branches are not equal"
    kill $server_process
    exit 1
fi
rama_branch_user_1=$(cat user1/.git/refs/heads/rama)
if [ ! -f user2/.git/refs/heads/rama ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
rama_branch_user_2=$(cat user2/.git/refs/heads/rama)
if [ "$rama_branch_user_1" != "$rama_branch_user_2" ]; then
    echo "❌ Failed. Branches are not equal"
    kill $server_process
    exit 1
fi

cd user2
if [ ! -f file1 ]; then
    echo "❌ Failed. File1 not found"
    kill $server_process
    exit 1
fi
../../../target/debug/git checkout rama
if [ ! -f file2 ]; then
    echo "❌ Failed. File2 not found"
    kill $server_process
    exit 1
fi
if [ ! -f file3 ]; then
    echo "❌ Failed. File3 not found"
    kill $server_process
    exit 1
fi

echo "✅ Pull con varias ramas"

sleep 1

cd -
../../target/debug/git clone git://127.1.0.0:9418/repo user3
if [ ! -f user3/.git/refs/heads/master ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi
if [ ! -f user3/.git/refs/heads/rama ]; then
    echo "❌ Failed. File not found"
    kill $server_process
    exit 1
fi

master_branch_user_3=$(cat user3/.git/refs/heads/master)
if [ "$master_branch_user_1" != "$master_branch_user_3" ]; then
    echo "❌ Failed. Branch references for 'master' are not equal"
    kill $server_process
    exit 1
fi

rama_branch_user_3=$(cat user3/.git/refs/heads/rama)
if [ "$rama_branch_user_1" != "$rama_branch_user_3" ]; then
    echo "❌ Failed. Branch references for 'rama' are not equal"
    kill $server_process
    exit 1
fi

cd user3
if [ ! -f file1 ]; then
    echo "❌ Failed. File1 not found"
    kill $server_process
    exit 1
fi
../../../target/debug/git checkout rama
if [ ! -f file2 ]; then
    echo "❌ Failed. File2 not found"
    kill $server_process
    exit 1
fi

if [ ! -f file3 ]; then
    echo "❌ Failed. File3 not found"
    kill $server_process
    exit 1
fi
cd ../../..
echo "✅ Clone con varias ramas"
kill $server_process
rm -rf server_client_integration_test
echo "✅ Passed"

