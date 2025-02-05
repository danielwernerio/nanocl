#!/bin/sh
## name: release_nanocl.sh

# variables

pkg_target_arg=$PKG_TARGET_ARCH
if [ -z "$pkg_target_arg" ]; then
  pkg_target_arg="amd64:x86_64-unknown-linux-musl"
fi
pkg_arch=$(echo ${pkg_target_arg} | awk -F: '{print $1}')
target_arch=$(echo ${pkg_target_arg} | awk -F: '{print $2}')
pkg_name="nanocl"
version=$(cat ./bin/nanocl/Cargo.toml | grep -m 1 "version = \"" | sed 's/[^0-9.]*\([0-9.]*\).*/\1/')
release_path="./target/${pkg_name}_${version}_${pkg_arch}"

echo "Building ${pkg_name} ${version} for ${pkg_arch} ${target_arch}"

# clear directory
rm -fr "${release_path}"
# create directories structure for package
mkdir -p "${release_path}"
mkdir -p "${release_path}"/DEBIAN
mkdir -p "${release_path}"/usr/bin
mkdir -p "${release_path}"/usr/share/man/man1

rustup target add ${target_arch}

# Build binary
cargo build --release --target=${target_arch} --features vendored --features release --bin nanocl

# Generate man pages
for file in ./bin/nanocl/target/man/*; do
  file_name=$(basename "${file}")
  gzip <"$file" >"${release_path}"/usr/share/man/man1/"$file_name".gz
done

# Copy binary
cp ./target/${target_arch}/release/${pkg_name} "${release_path}"/usr/bin
# Generate DEBIAN controll
cat >${release_path}/DEBIAN/control <<-EOM
Package: ${pkg_name}
Version: ${version}
Architecture: ${pkg_arch}
Maintainer: next-hat team@next-hat.com
Description: A self-sufficient vms and containers orchestrator
EOM

mkdir -p ./release/debian
mkdir -p ./release/linux
dpkg-deb --build --root-owner-group "${release_path}" ./release/debian/${pkg_name}_"${version}"_"${pkg_arch}".deb
rm -rf ${release_path}/DEBIAN
tar -czvf ./release/${pkg_name}_${version}_linux_${pkg_arch}.tar.gz -C ${release_path}/usr .
