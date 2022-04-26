FROM ubuntu:20.04
COPY Dockerfile /root/
CMD /usr/bin/cat /root/Dockerfile

