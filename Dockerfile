FROM ubuntu:22.04
COPY Dockerfile /root/
CMD /usr/bin/cat /root/Dockerfile

