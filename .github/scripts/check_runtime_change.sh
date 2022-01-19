grep "spec_version:" old-repo/bin/runtime/src/lib.rs | grep -o '[0-9]*' > old.version
grep "spec_version:" new-repo/bin/runtime/src/lib.rs | grep -o '[0-9]*' > new.version
diff old.version new.version
echo "::set-output name=diff::$?"
