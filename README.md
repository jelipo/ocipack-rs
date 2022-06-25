# ocipack-rs
一个可以快速构建 OCI/Docker 镜像的工具
## 使用场景举例
- 在无Docker，Containerd等Runtime的情况下，构建一个简单的镜像
- 使用CI时Docker in Docker编译了程序，需要COPY产物到Image中