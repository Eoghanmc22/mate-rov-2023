#!/bin/bash

	v4l2-ctl -d $1 --set-ctrl=focus_automatic_continuous=0
	v4l2-ctl -d $1 --set-ctrl=focus_absolute=0
