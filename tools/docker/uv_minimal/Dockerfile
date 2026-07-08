FROM harbor-eq-dc.sw-ci.kraken.as/dockerhub-proxy/python:3.12-slim

COPY --from=harbor-eq-dc.sw-ci.kraken.as/dockerhub-proxy/astral/uv:latest /uv /uvx /bin/
ARG PYTHON_VERSION=3.14
RUN uv python install ${PYTHON_VERSION}
ENV UV_PYTHON_INSTALL_REQUEST=auto

RUN apt-get update && apt-get -y install --no-install-recommends \
      git \
      git-lfs \
      openssh-client \
      curl \
      zip \
      unzip \
    && rm -rf /var/lib/apt/lists/*


# To enable github actions/checkout@v4 (This is a hack. TODO: Figure out a better solution)
RUN mkdir -m 1777 /__w && mkdir -m 1777 -p /runner/_work

CMD ["/bin/bash"]
