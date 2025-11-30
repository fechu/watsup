#!/usr/bin/env bash

w() {
    if [ -z "$WATSUP_BINARY" ]; then
        echo "Error: WATSUP_BINARY is not set."
        return 1
    fi

    # Ongoing project, show status and list of possible actions
	$WATSUP_BINARY log --current --no-pager --from $(date +%Y-%m-%d) --to $(date +%Y-%m-%d)
    action=$(echo -e "do-nothing\nstart\nstart-edit\nstop\nstop-edit\nabort\nedit\nchange\nhelp" | fzf --height=11 --prompt="Select an action: ")
    case $action in
        start*)
            project=$($WATSUP_BINARY projects | fzf --prompt="Select a project to start tracking: ")
            if [ -n "$project" ]; then
                $WATSUP_BINARY start "$project"
            else
                echo "No project selected."
            fi
            if [ "$action" = "start-edit" ]; then
                $WATSUP_BINARY edit
            fi
            ;;
        stop)
            $WATSUP_BINARY stop
            echo "Tracking stopped."
            ;;
        stop-edit)
            $WATSUP_BINARY stop
            $WATSUP_BINARY edit
            echo "Tracking stopped."
            ;;
        abort)
            $WATSUP_BINARY cancel
            echo "Tracking aborted."
            ;;
        edit)
            $WATSUP_BINARY edit
            ;;
        change)
            $WATSUP_BINARY stop
            $WATSUP_BINARY edit
            project=$($WATSUP_BINARY projects | fzf --prompt="Select a project to start tracking: ")
            $WATSUP_BINARY start --no-gap "$project"
            ;;
        help)
            echo ""
            echo "Shell function to make watsup interactive."
            echo ""
            echo "Works by wrapping some of the open used commands into selections (via fzf) and then executes the selected command."
            echo "Requires fzf (https://github.com/junegunn/fzf) to be installed and available in the current shell."
            echo ""
            echo "Usage:"
            echo "Source this file \"source "".sh\" and then run the shortcut \"w\""
            echo ""
            echo "Note: Sourcing this script will mask the w binary (https://man7.org/linux/man-pages/man1/w.1.html)"
            ;;
        do-nothing)
            ;;
        *)
            echo "No action selected."
            ;;
    esac
}
