#!/bin/sh
if [ ${GITHUB_REF} = 'refs/heads/prod' ];
then
  echo "tag=latest" >> ${GITHUB_OUTPUT}
else
  echo "tag=unstable" >> ${GITHUB_OUTPUT}
fi
