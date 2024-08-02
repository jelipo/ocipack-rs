<div align="center">
<br>
<h1>ocipack</h1>
<br>
Quickly build simple Docker/OCI images without runtime<br><br>

[![GitHub last commit](https://img.shields.io/github/last-commit/jelipo/ocipack-rs)](https://github.com/jelipo/ocipack-rs/commits)
[![GitHub release (latest by date)](https://img.shields.io/github/v/release/jelipo/ocipack-rs)](https://github.com/jelipo/ocipack-rs/releases)
[![GitHub all releases](https://img.shields.io/github/downloads/jelipo/ocipack-rs/total)](https://github.com/jelipo/ocipack-rs/releases)
![GitHub repo size](https://img.shields.io/github/repo-size/jelipo/ocipack-rs)
[![Github Release Publish Status](https://img.shields.io/github/actions/workflow/status/jelipo/ocipack-rs/rust.yml?branch=main)](https://github.com/jelipo/ocipack-rs/actions)
[![License](https://img.shields.io/github/license/jelipo/ocipack-rs)](https://github.com/jelipo/ocipack-rs/blob/master/LICENSE)

</div>

## Overview

The original purpose of `ocipack` was to build simple image without Runtime.

Now you can also:
- Pull image to a file without `Runtime`
- Display the image details of the image repository.
- Transform between `Docker` and `OCI` format.


## Download

### Linux / MacOS

```
curl -L https://github.com/jelipo/ocipack-rs/releases/download/0.7.2/ocipack-0.7.2-amd64_$(uname).tar.gz | tar xzv

# [Option] Move to /usr/local/bin/
sudo cp ocipack /usr/local/bin/ &&  sudo chmod +x /usr/local/bin/ocipack
```

### Windows

```
curl.exe -L https://github.com/jelipo/ocipack-rs/releases/download/0.7.2/ocipack-0.7.2-amd64_windows.zip -o ocipack.zip

tar -xf ocipack.zip
```

## How to use

Create a Dockerfile.

```bash
FROM ubuntu:24.04
COPY Dockerfile /root/
CMD cat /root/Dockerfile
```

Run this command.

```bash
ocipack build \
  --source=dockerfile:./Dockerfile \
  --target=registry:my.harbor.com/jelipo/demo:v1 \
  --target-auth=jelipo:my_password
```

Now a simple image has been pushed to the image repository.

```text
Build job successful!

Target image:
my.harbor.com/jelipo/demo:v1

```

You can also save the image as a file, just modify the `--target` parameter.

```bash
--target=tgz:demo.tgz
```


## Support Dockerfile?

No.

Because there is no runtime,
instructions such as `RUN`, `ARG`, and `MAINTAINER` in the image cannot be supported.

The example above uses Dockerfile because Dockerfile is more familiar to most people.

|             | instructions |
|:-----------:| :---: |
|   Support   | `FROM` `LABEL` `CMD` `COPY` `ENV` `USER` `WORKDIR` `EXPOSE` |
|    TODO     | `ADD` `ENTRYPOINT` `VOLUME` |
| Not support | `ARG` `RUN` `MAINTAINER` |

## Proxy

If you are in China or offline with a proxy provided,
you may not be able to `pull/push` images from the `Docker Hub` repository properly.

You can use the proxy to `pull/push` images.

```bash
ocipack build \
  --source=registry:redis:latest \
  --target=tgz:redis.tgz \
  --source-proxy=socks5://127.0.0.1:1080
  --target-proxy=http://name:pass@127.0.0.1:7890
```

## Platform

You can use the `--platform` parameter to set the platform of the image.

If not set, `linux/amd64` will be used by default.

## Transform


Transform between `Docker` and `OCI` format. Use `ocipack transform` .

```bash
ocipack transform --help
```


## Show Image Info

If you want to view the image information in the image warehouse, use this command.

```bash
me@jelipo:~$ ocipack show-info -i registry:alpine:latest
[23:18:26.011 INFO] Requesting registry...
[23:18:28.249 INFO] Platform is not set, use default platform linux/amd64.
[23:18:29.359 INFO] Request done.

IMAGE DETAILS

HOST            : registry-1.docker.io
IMAGE_NAME      : library/alpine
IMAGE_REFERENCE : latest
MANIFEST_TYPE   : Docker V2,Schema2
OS              : linux
ARCH            : amd64
CMD             : ["/bin/sh"]
ENTRYPOINT      : NONE

MANIFEST_LIST_PLATFORMS:
linux/amd64,
...

MANIFEST_LIST_RAW:
{...}

MANIFEST_RAW    :
{...}

CONFIG_BLOB_RAW :
{...}
```

## Clean Cache

```bash
ocipack clean -a
```


## Supported Manifest

|            version             | support |
|:------------------------------:|:-------:|
| `Image Manifest V 2, Schema 2` |    ✅    |
| `Image Manifest V 2, Schema 1` |    ❌    |
|      `OCI Image Manifest`      |    ✅    |


## Build

```bash
cargo build --release
```

## TODO

- Export or import to container engine.
- More Dockerfile instructions.

## Final

Thanks for using, please give feedback if you have any questions.
