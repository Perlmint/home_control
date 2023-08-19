from __future__ import annotations

import requests

from homeassistant.components.number import (
    NumberEntity,
    NumberDeviceClass,
)

class WaterPump(NumberEntity):
    def __init__(self, host):
        self._url = f'http://{host}/power/1'
        self._name = f'{host} water pump'
        self._value = None

    @property
    def device_class(self) -> str:
        return NumberDeviceClass.POWER_FACTOR 

    @property
    def mode(self) -> str:
        return "slider"

    @property
    def native_step(self) -> float:
        return 1

    @property
    def native_value(self):
        return self._value

    def update(self) -> None:
        self._value = int(requests.get(self._url).text)

    def set_native_value(self, value: float) -> None:
        self._value = int(requests.put(self._url, 255 * value / 255.0))