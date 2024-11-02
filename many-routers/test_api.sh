#!/bin/bash

BASE_URL="http://127.0.0.1:3030"
SERVER_PID=""

function start_server() {
    if [ -z "$SERVER_PID" ]; then
        cargo run &
        SERVER_PID=$!
        echo "服务器已启动，PID: $SERVER_PID"
    else
        echo "服务器已经在运行，PID: $SERVER_PID"
    fi
}

function stop_server() {
    if [ -n "$SERVER_PID" ]; then
        kill $SERVER_PID
        echo "服务器已停止"
        SERVER_PID=""
    else
        echo "服务器未运行"
    fi
}

function clean_server() {
    cargo clean
    echo "服务器已清理"
}

function add_question() {
    read -p "请输入问题ID: " id
    read -p "请输入问题标题: " title
    read -p "请输入问题内容: " content
    read -p "请输入问题标签(用逗号分隔): " tags

    curl -X POST "$BASE_URL/questions" \
    -H "Content-Type: application/json" \
    -d "{
          \"id\": \"$id\",
          \"title\": \"$title\",
          \"content\": \"$content\",
          \"tags\": [$(echo $tags | sed 's/,/\",\"/g' | sed 's/^/\"/' | sed 's/$/\"/')]
        }"
}

function add_answer() {
    read -p "请输入答案内容: " content
    read -p "请输入问题ID: " question_id

    curl -X POST "$BASE_URL/comments" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "content=$content&questionId=$question_id"
}

function get_all_questions() {
    curl -X GET "$BASE_URL/questions"
}

function get_all_comments() {
    curl -X GET "$BASE_URL/comments"
}

function get_comments_by_question_id() {
    read -p "请输入问题ID: " question_id
    curl -X GET "$BASE_URL/questions/$question_id/comments"
}

function show_menu() {
    echo "请选择操作:"
    echo "1) 运行服务器"
    echo "2) 停止服务器"
    echo "3) 清理服务器"
    echo "4) 添加问题"
    echo "5) 添加答案"
    echo "6) 获取所有问题"
    echo "7) 获取所有评论"
    echo "8) 获取特定问题的评论"
    echo "9) 退出"
}

while true; do
    show_menu
    read -p "请输入选项: " choice
    case $choice in
        1)
            start_server
            ;;
        2)
            stop_server
            ;;
        3)
            clean_server
            ;;
        4)
            add_question
            ;;
        5)
            add_answer
            ;;
        6)
            get_all_questions
            ;;
        7)
            get_all_comments
            ;;
        8)
            get_comments_by_question_id
            ;;
        9)
            stop_server
            break
            ;;
        *)
            echo "无效选项，请重新选择"
            ;;
    esac
    echo ""
done