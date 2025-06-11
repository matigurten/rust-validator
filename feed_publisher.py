import asyncio
import json
import random
import time
from nats.aio.client import Client as NATS

async def main():
    nc = NATS()
    await nc.connect("nats://nats:4222")  # Use service name 'nats' for Docker networking

    symbols = ["AAPL", "TSLA"]  # Reduced to two symbols
    actions = ["Buy", "Sell"]
    order_types = ["Market", "Limit", "Cancel"]
    sequence = 1

    while True:
        action = random.choice(actions)
        order_type = random.choices(order_types, weights=[0.45, 0.45, 0.1])[0]  # Fewer cancels
        amount = round(random.uniform(1, 100), 2)
        price = round(random.uniform(100, 500), 2)
        order = {
            "id": 0,  # or use now_ns if you want
            "symbol": random.choice(symbols),
            "price": price,
            "amount": amount,
            "action": action,
            "order_type": order_type,
            # timestamp will be set below
        }
        now_ns = time.time_ns()  # <--- sample here, right before publish
        order["timestamp"] = now_ns
        await nc.publish("market_data", json.dumps(order).encode())
        print("Published:", order)
        await asyncio.sleep(2)  # Increased wait time for less load

    await nc.drain()

if __name__ == "__main__":
    asyncio.run(main())