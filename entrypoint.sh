#!/bin/sh
while [ 1 ];
do
    /app/diesel database setup && break;
done
/app/diesel migration run
/app/testaustime-rs
