import pytest

from harness import ServerHarness


@pytest.fixture
def harness(tmp_path, request):
    instances = []

    def factory(**kwargs):
        instance = ServerHarness(tmp_path / f"instance-{len(instances)}", **kwargs)
        instances.append(instance)
        return instance

    def teardown():
        for instance in reversed(instances):
            instance.stop()

    request.addfinalizer(teardown)
    return factory


@pytest.fixture
def server(harness):
    return harness()


@pytest.fixture
def client(server):
    client = server.new_client()
    yield client
    client.close()
