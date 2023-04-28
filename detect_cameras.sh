#!/bin/bash
#
for dev in $(v4l2-ctl --list-devices | grep "/dev/video" | xargs echo)
do
	v4l2-ctl -d $dev --list-formats | grep "Video Capture$" > /dev/null || continue
	v4l2-ctl -d $dev --list-formats | grep "H264" > /dev/null || continue
	v4l2-ctl -d $dev --set-ctrl=focus_automatic_continuous=0
	v4l2-ctl -d $dev --set-ctrl=focus_absolute=0
	echo $dev
done
