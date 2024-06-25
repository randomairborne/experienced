#!/bin/sh
if [ ${GITHUB_REF} = 'prod' ];
then
  echo "tag=latest" >> ${GITHUB_OUTPUT}
else
  echo "tag=unstable" >> ${GITHUB_OUTPUT}
fi
