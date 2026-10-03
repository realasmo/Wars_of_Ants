#!/bin/bash
cd ~/AI/GLM-projects/__Android/Games/Wars_of_Ants/client

# only the process LISTENING on the port — plain `lsof -i:5173` also lists
# every connected client (your open game tab), and kill would hit those too
function server_pid() {
	lsof -t -i:5173 -sTCP:LISTEN
}

function stop() {
	gameserver_pid=$(server_pid)
	if [[ ! -z ${gameserver_pid} ]]; then
		echo "killing $gameserver_pid"
		kill ${gameserver_pid}
		# give it up to 2s to exit cleanly (frees the port for --strictPort)
		for i in {1..20}; do
			[[ -z $(server_pid) ]] && break
			sleep 0.1
		done
		if [[ ! -z $(server_pid) ]]; then
			echo "not exiting - kill -9 $(server_pid)"
			kill -9 $(server_pid)
		fi
	  else
		echo "nothing to stop, pid not found"
	fi
}

function start() {
	echo "starting ant server - url: http://localhost:5173"
	# nohup + disown: survives closing the terminal; log in /tmp/woa-vite.log
	nohup npm run dev -- --port 5173 --strictPort > /tmp/woa-vite.log 2>&1 &
	disown
}

function status() {
	gameserver_pid=$(server_pid)
	if [[ ! -z "${gameserver_pid}" ]]; then
		echo "pid: ${gameserver_pid} - running."
	  else
		echo "not running"
	fi
}

function restart() {
	status
	stop
	start
	sleep 2
	status
}

function rebuild() {
	cd ~/AI/GLM-projects/__Android/Games/Wars_of_Ants/client && npm run wasm
}

if [[ $1 = "stop" ]]; then
	stop
  elif [[ $1 = "start" ]]; then
	start
  elif [[ $1 = "status" ]]; then
	status
  elif [[ $1 = "restart" ]]; then
	restart
  elif [[ $1 = "rebuild" ]]; then
	rebuild
  else echo "usage: $0 <start|stop|status|restart|rebuild>"
	exit 0
fi
