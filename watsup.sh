#!/usr/bin/env bash

w() {
    # Ongoing project, show status and list of possible actions
	watson log --current --no-pager --from $(date +%Y-%m-%d) --to $(date +%Y-%m-%d)
    action=$(echo -e "do-nothing\nstart\nstart-edit\nstop\nstop-edit\nabort\nedit\nchange\nhelp" | fzf --height=11 --prompt="Select an action: ")
    case $action in
        start-*)
            project=$(watson projects | fzf --prompt="Select a project to start tracking: ")
            if [ -n "$project" ]; then
                watson start "$project"
            else
                echo "No project selected."
            fi
            if [ "$action" = "start-edit" ]; then
                watson edit
            fi
            ;;
        stop)
            watson stop
            echo "Tracking stopped."
            ;;
        stop-edit)
            watson stop
           	watson edit
            echo "Tracking stopped."
            ;;
        abort)
            watson cancel
            echo "Tracking aborted."
            ;;
        edit)
            watson edit
            ;;
        change)
           	watson stop
           	watson edit
            project=$(watson projects | fzf --prompt="Select a project to start tracking: ")
            watson start --no-gap "$project"
           	;;
        help)
            echo ""
            echo "Shell function to make watsup interactive."
            echo ""
            echo "Works by wrapping some of the open used commands into selections (via fzf) and then executes the selected command."
            echo "Requires fzf (https://github.com/junegunn/fzf) to be installed and available in the current shell."
            echo ""
            echo "Usage:"
            echo "Source this file \"source watsup.sh\" and then run the shortcut \"w\""
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
