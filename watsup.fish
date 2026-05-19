function w
    if not set -q WATSUP_BINARY
        echo "Error: WATSUP_BINARY is not set."
        return 1
    end

    # Ongoing project, show status and list of possible actions
    $WATSUP_BINARY log --current --from (date +%Y-%m-%d) --to (date +%Y-%m-%d)
    set action (printf "do-nothing\nstart\nstart-edit\nstop\nstop-edit\nabort\nedit\nchange\nhelp" | fzf --height=11 --prompt="Select an action: ")
    switch $action
        case 'start*'
            set project ($WATSUP_BINARY projects | fzf --prompt="Select a project to start tracking: ")
            if test -n "$project"
                $WATSUP_BINARY start "$project"
            else
                echo "No project selected."
            end
            if test "$action" = start-edit
                $WATSUP_BINARY edit
            end
        case stop
            $WATSUP_BINARY stop
            echo "Tracking stopped."
        case stop-edit
            $WATSUP_BINARY stop
            $WATSUP_BINARY edit
            echo "Tracking stopped."
        case abort
            $WATSUP_BINARY cancel
            echo "Tracking aborted."
        case edit
            set frame_line ($WATSUP_BINARY log --current --from 2000-01-01 | grep -E '^\s+[0-9a-f]+' | tail -20 | fzf --prompt="Select a frame to edit: ")
            if test -n "$frame_line"
                set frame_id (string trim $frame_line | string split -f1 ' ')
                $WATSUP_BINARY edit $frame_id
            else
                echo "No frame selected."
            end
        case change
            $WATSUP_BINARY stop
            $WATSUP_BINARY edit
            set project ($WATSUP_BINARY projects | fzf --prompt="Select a project to start tracking: ")
            $WATSUP_BINARY start --no-gap "$project"
        case help
            echo ""
            echo "Shell function to make watsup interactive."
            echo ""
            echo "Works by wrapping some of the open used commands into selections (via fzf) and then executes the selected command."
            echo "Requires fzf (https://github.com/junegunn/fzf) to be installed and available in the current shell."
            echo ""
            echo "Usage:"
            echo "Source this file \"source watsup.fish\" and then run the shortcut \"w\""
            echo ""
            echo "Note: This function will mask the w binary (https://man7.org/linux/man-pages/man1/w.1.html)"
        case do-nothing
        case '*'
            echo "No action selected."
    end
end
