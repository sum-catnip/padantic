#!/usr/bin/env python3

import requests

while True:
    url = input()
    if not url: break
    if requests.get(f'http://127.0.0.1/{url}').status_code == 200:
        print('yes')
    else: print('no')

