<div align="center">
<br>
<h1>ocipack</h1>
<br>
一个可以快速构建 OCI/Docker 镜像的工具<br><br>

[![GitHub last commit](https://img.shields.io/github/last-commit/jelipo/ocipack-rs)](https://github.com/jelipo/ocipack-rs/commits)
[![GitHub release (latest by date)](https://img.shields.io/github/v/release/jelipo/ocipack-rs)](https://github.com/jelipo/ocipack-rs/releases)
[![GitHub all releases](https://img.shields.io/github/downloads/jelipo/ocipack-rs/total)](https://github.com/jelipo/ocipack-rs/releases)
![GitHub repo size](https://img.shields.io/github/repo-size/jelipo/ocipack-rs)
[![Github Release Publish Status](https://img.shields.io/github/workflow/status/jelipo/ocipack-rs/Rust)](https://github.com/jelipo/ocipack-rs/actions)
[![License](https://img.shields.io/github/license/jelipo/ocipack-rs)](https://github.com/jelipo/ocipack-rs/blob/master/LICENSE)




</div>

## 特性
- 在没有`Docker/Containerd`等容器引擎环境下构建简单镜像
- 兼容 Dockerfile 大部分配置项，降低上手难度
- 支持 OCI 和 Docker 镜像格式，并支持互相转换
- 支持镜像`Pull和Push`时使用 `socks5/http` 代理
- 一个单独的二进制，Linux下静态编译无依赖，alpine也可使用
- 支持 `Windows` `MacOS` `Linux` 三个平台的使用
- 支持性能非常好的`zstd`解压缩算法

## 下载

### Linux 和 MacOS

```
curl -L https://github.com/jelipo/ocipack-rs/releases/download/0.3.0/ocipack-0.3.0-amd64_$(uname).tar.gz | tar xzv
```

把下载解压完成的 `ocipack` 放到 `/usr/local/bin/` （可选操作）

```
cp ocipack /usr/local/bin/ &&  sudo chmod +x /usr/local/bin/ocipack
```

### Windows

```
curl.exe -L https://github.com/jelipo/ocipack-rs/releases/download/0.3.0/ocipack-0.3.0-amd64_windows.zip -o ocipack.zip

tar -xf ocipack.zip
```

## 简介

作者在学习云原生和写代码的时候，经常需要构建一个简单的镜像，但是有时候会因为各种原因导致并不轻松。

- 着急开发，但是没有`Docker/Containerd`等环境。
- 居家办公需要连接VPN到组织的网络中，但是 Windows 和 MacOS 使用虚拟机运行`Docker`，这意味着虚拟机中的`Docker`无法通过宿主的VPN网络`Pull和Push`镜像。
- `Linux`服务器上，`Docker`/`Containerd` 等引擎在构建时拉取公共镜像因为众所周知的原因速度非常慢。即使有`socks5/http`
  代理，但是服务器上可能还有正在运行的容器化进程，配置代理意味着重启，且整个容器引擎都会走代理，一般是不可接受的，况且频繁配置也很麻烦。
- `CI/CD`环境中，你可能可以使用`Docker多阶段构建`、`CI工具提供的环境`构建一个镜像并打包成`Image`并上传到`Registry`中。<br>
  通常这是两个步骤:`构建产物`和`构建成镜像并Push`，但是有时候CI环境并不如我们的意(可能没有容器环境、只有`Docker in Docker`无法Push 等问题)，而且需要学习每个CI环境来完成我们的这两个步骤。<br>
  如果有一个通用的工具可以把`产物`构建成镜像并Push到`Registry`就可以大大提升我们对不同CI环境的兼容。

针对以上问题，所以写了一个小工具去解决这些问题。<br>
也有很多别的工具可以解决，重复造轮子的目的，也是为了加深Rust的编写能力和Image知识。<br>

## 限制

因为本工具没有任何Runtime，所以Dockerfile配置项中需要Runtime支持的一律无法正常支持。<br>
`为了便于上手使用，本工具只是使用常见的Dockerfile作为配置，降低使用门槛，但并不是完全兼容Dockerfile`<br>
|  | Filed |
| :---: | :---: |
| 支持的配置项 | `FROM` `LABEL` `CMD` `COPY` `ENV` `USER` `WORKDIR` `EXPOSE` |
| 暂不支持但未来会支持| `ADD`(可以暂时用COPY代替) `ENTRYPOINT` `VOLUME` |
| 不会支持 | `ARG` `RUN` `MAINTAINER` |
| 多阶段构建 | 不支持 |

<br>

目前的Image Manifest主流为Docker，但是Docker Manifest格式也分为多个版本，本工具对于Docker格式只支持`Image Manifest V 2, Schema 2` ，对于老版本的`Image Manifest V 2, Schema 1` 不支持，也不计划进行支持。

| 版本 | 是否支持 |
| :---: | :---: |
| `Image Manifest V 2, Schema 2` | ✅  |
| `Image Manifest V 2, Schema 1` | ❌ |
| `OCI Image Manifest` | ✅ |


## 使用举例

很多时候，我们需要的功能只是把文件COPY进`Base Image`，然后运行。

创建一个`Dockerfile`文件，这个`Dockerfile`既是我们构建Image的配置文件，也是我们计划`run image`时打印内容的文件。写入以下内容:

```
FROM ubuntu:22.04
COPY Dockerfile /root/
CMD cat /root/Dockerfile
```

接着运行以下命令：

```bash
./ocipack build \
  --source=dockerfile:./Dockerfile \
  --target=registry:my.harbor.com/jelipo/demo:v1 \
  --target-auth=jelipo:my_password
```

如果顺利，末尾将会输出:
```
Build job successful!

Target image:
my.harbor.com/jelipo/demo:v1

```
当看到`Build job successful`字样时，说明我们已经构建完成并上传到了`Registry`。

接下来我们执行`docker run -it --rm my.harbor.com/jelipo/demo:v1`时会输出我们的`Dockerfile`内容。


## 功能

## 构建（Build）

最主要的功能，拉取Base Image，然后把文件COPY进Image，然后Push。<br>
使用`ocipack build`子命令。

```bash
# 当base image的registry为http而非https时需要启用
-a, --allow-insecure 
        Allow insecure registry 
# 连接超时时间
    --conn-timeout <CONN_TIMEOUT>
        [OPTION] Connection timeout in seconds [default: 600]
# 新Image的格式
-f, --format <FORMAT>
        [OPTION] Target format type. Support 'docker' and 'oci' [default: docker]
# 指定Dockerfile的路径
-s, --source <SOURCE>
        Source type. Support dockerfile type Example:'dockerfile:/path/to/.Dockerfile'
# Base Image的auth验证信息，支持环境变量
    --source-auth <SOURCE_AUTH>
        [OPTION] Auth of pull source image. Example:'myname:mypass','myname:${MY_PASSWORD_ENV}'
# Base Image的代理信息
    --source-proxy <SOURCE_PROXY>
        [OPTION] Proxy of pull source image.
        Example:'socks5://127.0.0.1:1080','http://name:pass@example:8080'
 # Target Image的信息
-t, --target <TARGET>
        Target type. Support 'registry' Example:'registry:my.reg.com/target/image:1.1'
# 当Target Image的Registry为http而非https时需要启用
    --target-allow-insecure
        Allow target insecure registry 
# Target Image的auth验证信息，支持环境变量
    --target-auth <TARGET_AUTH>
        [OPTION] Auth of push target image. Example:'myname:mypass','myname:${MY_PASSWORD_ENV}'
# Target Image的代理信息
    --target-proxy <TARGET_PROXY>
        [OPTION] Proxy of push target image.
        Example:'socks5://127.0.0.1:1080','http://name:pass@example:8080'
# 新layer使用zstd压缩，zstd拥有更好的 解压缩速度 和 压缩比（很多Runtime不支持zstd，谨慎使用）
    --use-zstd
        [OPTION] Compress files using zstd
```


## 可能遇到的问题

### Base Image 拉取非常慢

如果是`hub.docker.com`中的公共镜像，因为网络原因本身就很慢，可以使用`--source-proxy`设置一个代理加速访问。
<br><br>

### Pull 或 Push 的 Registry 不支持HTTPS协议

目前大多数容器引擎或者工具都会默认使用`HTTPS`访问`Registry`，但是很多内网或者学习使用时会使用`HTTP`协议的 Registry 。

当Pull镜像的Registry是`HTTP`协议添加`--allow-insecure`选项。

如果Push的Registry是`HTTP`协议，添加`--target-allow-insecure`选项。
<br><br>

### 执行命令后，在Linux history中会显示我的密码

工具支持从环境变量中读取密码，可以在`build`子命令的help中看`--source-auth`或者`--target-auth`的说明。
<br><br>

### 构建出来的Image是Docker Manifest格式的，想要OCI格式的

添加`--format=oci`选项。
<br><br>

## 转换(Transform)

此功能主要是为了 Docker和OCI 之间的转换。主要命令跟`build`子命令大同小异，可以参考上面的`构建（Build）`。<br>
可以使用`ocipack build -h`查看详情。<br><br>

## 清理缓存(Clean)
因为无论Pull还是Push，都会需要暂存文件在本地中，一边下次Pull加速。

可以使用`ocipack clean`子命令清理本地的缓存文件夹，可以使用`ocipack clean -h`查看更多参数选项。
<br>
<br>
## 挖坑
- 支持导出或者导入本地的容器引擎。
- 更多的Dockerfile配置项。
- 创建新Image时提供 使用`zstd`压缩所有layer。
<br>

## 最后
本工具目前属于个人开发使用阶段，虽然基本功能自测没有问题，但是还没有稳定，不建议在重要环境使用。
