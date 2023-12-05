echo "Generic tests"
RUSTFLAGS="-Awarnings" cargo test -q -p git -p git-lib -p git-server

# Run log_integration.rs test
echo "=========================="
echo "Log Integration test"
unzip -qq git/tests/data/commands/log.zip -d git/tests/data/commands/
RUSTFLAGS="-Awarnings" cargo test -q -p git -p git-lib -p git-server --test log_integration -- --ignored

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